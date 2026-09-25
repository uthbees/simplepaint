//! This module handles interactions with the GPU that are too complex to fit into app.rs.

use crate::canvas::canvas_buffer::CanvasBuffer;
use crate::ui::canvas_panel::CanvasPanel;
use bytemuck::{Pod, Zeroable};
use egui_wgpu::RenderState;
use wgpu::PipelineCompilationOptions;
use wgpu::util::DeviceExt;

/// Camera parameters passed to the canvas shader.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    /// Pan amount in a percentage of the canvas panned (with the origin being the center of the canvas).
    /// For example, if the bottom right corner of the canvas was in the center of the screen,
    /// `pan` would be [0.5, 0.5].
    pan: [f32; 2],
    zoom: f32,
    _padding: f32,
    /// Factors to multiply with the starting uv to normalize the canvas to the proper aspect ratio
    /// and a 1:1 canvas pixel:screen pixel scale.
    normalization_factors: [f32; 2],
    _padding2: [f32; 2],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            pan: [0.0, 0.0],
            zoom: 1.0,
            _padding: 0.0,
            normalization_factors: [1.0, 1.0],
            _padding2: [0.0, 0.0],
        }
    }
}

/// All GPU resources that are tied to the canvas texture and canvas rendering pipeline.
pub struct CanvasGpu {
    /// The raw canvas texture written to by CPU drawing operations.
    raw_texture: wgpu::Texture,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    /// Final canvas panel texture, displaying the view of the canvas from the camera.
    panel_texture: wgpu::Texture,
    /// The panel texture's id registered with egui.
    pub panel_texture_id: egui::TextureId,
    panel_texture_view: wgpu::TextureView,
    panel_width: u32,
    panel_height: u32,
}

impl CanvasGpu {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    /// Create the textures, shader pipeline, uniform buffer, and register offscreen target with egui.
    pub fn new(render_state: &RenderState, width: u32, height: u32, pixels: &[u8]) -> Self {
        let device = &render_state.device;
        let queue = &render_state.queue;

        let raw_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raw_canvas_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        CanvasGpu::write_texture(queue, &raw_texture, width, height, pixels);
        let raw_texture_view = raw_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("canvas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let camera_uniform = CameraUniform::default();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("canvas_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("canvas_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&raw_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/canvas_panel.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("canvas_render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let panel_width = width.max(1);
        let panel_height = height.max(1);

        let (panel_texture, panel_texture_view) =
            Self::create_panel_texture(device, panel_width, panel_height);

        let panel_texture_id = render_state.renderer.write().register_native_texture(
            device,
            &panel_texture_view,
            wgpu::FilterMode::Nearest,
        );

        Self {
            raw_texture,
            camera_uniform,
            camera_buffer,
            bind_group,
            pipeline,
            panel_texture,
            panel_texture_id,
            panel_texture_view,
            panel_width,
            panel_height,
        }
    }

    /// Re-upload the full CPU canvas buffer to the raw GPU texture.
    pub fn upload_canvas_state(&self, queue: &wgpu::Queue, canvas: &CanvasBuffer) {
        CanvasGpu::write_texture(
            queue,
            &self.raw_texture,
            canvas.width(),
            canvas.height(),
            canvas.as_bytes(),
        );
    }

    /// Updates the `CameraUniform` and uploads the updated data.
    /// Returns the computed aspect ratio correction.
    pub fn update_camera(
        &mut self,
        queue: &wgpu::Queue,
        canvas_panel_state: &CanvasPanel,
    ) -> [f32; 2] {
        self.camera_uniform = CameraUniform {
            pan: [
                canvas_panel_state.pan_px.x / canvas_panel_state.canvas.width() as f32,
                canvas_panel_state.pan_px.y / canvas_panel_state.canvas.height() as f32,
            ],
            zoom: canvas_panel_state.zoom,
            _padding: 0.0,
            normalization_factors: canvas_panel_state.get_normalization_factors(),
            _padding2: [0.0, 0.0],
        };

        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera_uniform),
        );

        [1.0, 1.0]
    }

    #[allow(clippy::cast_sign_loss)]
    /// Handles changes to the requested canvas panel size.
    pub fn update_panel_size(
        &mut self,
        render_state: &RenderState,
        requested_width: f32,
        requested_height: f32,
    ) {
        let requested_width = requested_width.max(1.0) as u32;
        let requested_height = requested_height.max(1.0) as u32;
        if requested_width == self.panel_width && requested_height == self.panel_height {
            return;
        }

        let device = &render_state.device;
        (self.panel_texture, self.panel_texture_view) =
            Self::create_panel_texture(device, requested_width, requested_height);

        render_state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                device,
                &self.panel_texture_view,
                wgpu::FilterMode::Nearest,
                self.panel_texture_id,
            );

        self.panel_width = requested_width;
        self.panel_height = requested_height;
    }

    /// Utility function to create a texture for the canvas panel.
    fn create_panel_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let panel_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas_panel_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let panel_texture_view = panel_texture.create_view(&wgpu::TextureViewDescriptor::default());

        (panel_texture, panel_texture_view)
    }

    /// Render the canvas texture with the camera uniform onto the canvas panel texture.
    pub fn render_canvas_panel_texture(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("canvas_render_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.panel_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.15,
                            g: 0.15,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Wrapper function for `queue.write_texture`.
    fn write_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        bytes: &[u8],
    ) {
        queue.write_texture(
            texture.as_image_copy(),
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}
