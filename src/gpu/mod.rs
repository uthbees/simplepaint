//! This module is responsible for managing complex interactions with the GPU. It is not
//! responsible for window or frame management.

use crate::canvas::canvas_buffer::CanvasBuffer;
use crate::ui::canvas_panel::CanvasPanel;
use bytemuck::{Pod, Zeroable};
use egui_wgpu::RenderState;
use std::iter;
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

struct LayerResources {
    texture: wgpu::Texture,
    composition_bind_group: wgpu::BindGroup,
}

/// All GPU resources that are tied to the canvas textures and rendering pipeline.
pub struct CanvasGpu {
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    layer_sampler: wgpu::Sampler,
    layer_composition_bind_group_layout: wgpu::BindGroupLayout,
    layer_textures: Vec<LayerResources>,
    layers_dirty: bool,
    composite_canvas_pipeline: wgpu::RenderPipeline,
    composite_canvas_texture_view: wgpu::TextureView,
    canvas_view_bind_group: wgpu::BindGroup,
    canvas_view_pipeline: wgpu::RenderPipeline,
    /// The texture view for the final canvas view, displaying the view of the canvas from the camera.
    canvas_view_texture_view: wgpu::TextureView,
    /// The canvas view texture's id registered with egui.
    canvas_view_texture_id: egui::TextureId,
    canvas_view_width: u32,
    canvas_view_height: u32,
}

impl CanvasGpu {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    /// Create the textures, shader pipeline, uniform buffer, and register offscreen target with egui.
    pub fn new(
        render_state: &RenderState,
        canvas_width: u32,
        canvas_height: u32,
        initial_layer_pixels: &[u8],
        canvas_view_width: u32,
        canvas_view_height: u32,
    ) -> Self {
        let device = &render_state.device;
        let queue = &render_state.queue;

        let canvas_width = canvas_width.max(1);
        let canvas_height = canvas_height.max(1);
        let canvas_view_width = canvas_view_width.max(1);
        let canvas_view_height = canvas_view_height.max(1);

        let layer_composition_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("layer_composition_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let layer_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("layer_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let layer_textures: Vec<_> = vec![];

        let composite_canvas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("composite_canvas_texture"),
            size: wgpu::Extent3d {
                width: canvas_width,
                height: canvas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let composite_canvas_texture_view =
            composite_canvas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let composite_canvas_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("composite_canvas_pipeline_layout"),
                bind_group_layouts: &[Some(&layer_composition_bind_group_layout)],
                immediate_size: 0,
            });

        // Load your WGSL shader (assumes you have a shader file)
        let composite_canvas_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/composite_canvas.wgsl"));

        let composite_canvas_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("composite_canvas_pipeline"),
                layout: Some(&composite_canvas_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &composite_canvas_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &composite_canvas_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: composite_canvas_texture.format(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let canvas_view_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("canvas_view_sampler"),
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

        let canvas_view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("canvas_view_bind_group_layout"),
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

        let canvas_view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("canvas_view_bind_group"),
            layout: &canvas_view_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&composite_canvas_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&canvas_view_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });

        let canvas_view_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas_view_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/canvas_view.wgsl").into()),
        });

        let canvas_view_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("canvas_view_pipeline_layout"),
                bind_group_layouts: &[Some(&canvas_view_bind_group_layout)],
                immediate_size: 0,
            });

        let canvas_view_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("canvas_view_pipeline"),
            layout: Some(&canvas_view_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &canvas_view_shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &canvas_view_shader_module,
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

        let canvas_view_texture_view =
            Self::create_canvas_view_texture(device, canvas_view_width, canvas_view_height);

        let canvas_view_texture_id = render_state.renderer.write().register_native_texture(
            device,
            &canvas_view_texture_view,
            wgpu::FilterMode::Nearest,
        );

        let mut canvas_gpu = Self {
            camera_uniform,
            camera_buffer,
            layer_sampler,
            layer_composition_bind_group_layout,
            layer_textures,
            layers_dirty: false,
            composite_canvas_pipeline,
            composite_canvas_texture_view,
            canvas_view_bind_group,
            canvas_view_pipeline,
            canvas_view_texture_view,
            canvas_view_texture_id,
            canvas_view_width,
            canvas_view_height,
        };

        let base_layer = canvas_gpu.create_layer_resources(device, canvas_width, canvas_height);
        CanvasGpu::write_texture(
            queue,
            &base_layer.texture,
            canvas_width,
            canvas_height,
            initial_layer_pixels,
        );
        canvas_gpu.layer_textures.push(base_layer);
        canvas_gpu.update_composite_canvas(device, queue);

        canvas_gpu
    }

    pub fn get_view_texture_id(&self) -> egui::TextureId {
        self.canvas_view_texture_id
    }

    /// Re-upload a layer's full CPU buffer to its raw GPU texture.
    pub fn upload_layer(
        &mut self,
        queue: &wgpu::Queue,
        layer_buffer: &CanvasBuffer,
        layer_index: usize,
    ) {
        if layer_index >= self.layer_textures.len() {
            eprintln!("Error: layer_index out of bounds in upload_layer");
            return;
        }

        CanvasGpu::write_texture(
            queue,
            &self.layer_textures[layer_index].texture,
            layer_buffer.width(),
            layer_buffer.height(),
            layer_buffer.as_bytes(),
        );
        self.layers_dirty = true;
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
    /// Handles changes to the requested canvas view size.
    pub fn update_canvas_view_size(
        &mut self,
        render_state: &RenderState,
        requested_width: f32,
        requested_height: f32,
    ) {
        let requested_width = requested_width.max(1.0) as u32;
        let requested_height = requested_height.max(1.0) as u32;
        if requested_width == self.canvas_view_width && requested_height == self.canvas_view_height
        {
            return;
        }

        let device = &render_state.device;
        self.canvas_view_texture_view =
            Self::create_canvas_view_texture(device, requested_width, requested_height);

        render_state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                device,
                &self.canvas_view_texture_view,
                wgpu::FilterMode::Nearest,
                self.canvas_view_texture_id,
            );

        self.canvas_view_width = requested_width;
        self.canvas_view_height = requested_height;
    }

    fn create_layer_resources(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> LayerResources {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas_layer_texture"),
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

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let composition_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("layer_composition_bind_group"),
            layout: &self.layer_composition_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.layer_sampler),
                },
            ],
        });

        LayerResources {
            texture,
            composition_bind_group,
        }
    }

    /// Re-render the composite canvas texture from the layer textures.
    fn update_composite_canvas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("composite_canvas_render_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite_canvas_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite_canvas_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear the canvas before drawing the bottom layer.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
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

            render_pass.set_pipeline(&self.composite_canvas_pipeline);

            for LayerResources {
                composition_bind_group,
                ..
            } in &self.layer_textures
            {
                render_pass.set_bind_group(0, composition_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }

        queue.submit(iter::once(encoder.finish()));
    }

    /// Utility function to create a texture for the canvas view.
    fn create_canvas_view_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> wgpu::TextureView {
        let view_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas_view_texture"),
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

        view_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Render the canvas view texture from the canvas texture with the camera uniform.
    pub fn render_canvas_view_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.layers_dirty {
            self.update_composite_canvas(device, queue);
            self.layers_dirty = false;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("canvas_view_render_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas_view_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.canvas_view_texture_view,
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

            render_pass.set_pipeline(&self.canvas_view_pipeline);
            render_pass.set_bind_group(0, &self.canvas_view_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));
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
