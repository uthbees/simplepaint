use crate::dimensions::{DimensionsPx, DimensionsPxNonzero};
use crate::gpu::camera::Camera;
use crate::gpu::composite_canvas::CompositeCanvas;
use crate::gpu::layers::Layers;
use crate::ui::ui_state::EnabledLayersConfig;
use egui_wgpu::RenderState;
use std::iter;
use wgpu::PipelineCompilationOptions;

/// The canvas view is the view of the composite canvas that the user sees, taking into account
/// factors such as pan and zoom. The canvas view texture is what is actually rendered by egui.
pub struct CanvasView {
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    texture_view: wgpu::TextureView,
    /// The canvas view texture's id registered with egui.
    texture_id: egui::TextureId,
    dimensions: DimensionsPxNonzero,
}

impl CanvasView {
    #[allow(clippy::too_many_lines)]
    pub fn new(
        render_state: &RenderState,
        camera: &Camera,
        composite_canvas: &CompositeCanvas,
        canvas_dimensions: DimensionsPxNonzero,
        canvas_view_dimensions: DimensionsPxNonzero,
    ) -> Self {
        let device = &render_state.device;

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("canvas_view_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("canvas_view_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(composite_canvas.texture_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera.get_buffer_binding(),
                },
            ],
        });

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas_view_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/canvas_view.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas_view_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("canvas_view_pipeline"),
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

        let texture_view = Self::create_canvas_view_texture(device, canvas_dimensions);

        let texture_id = render_state.renderer.write().register_native_texture(
            device,
            &texture_view,
            wgpu::FilterMode::Nearest,
        );

        Self {
            bind_group,
            pipeline,
            texture_view,
            texture_id,
            dimensions: canvas_view_dimensions,
        }
    }

    pub fn texture_id(&self) -> egui::TextureId {
        self.texture_id
    }

    #[allow(clippy::cast_sign_loss)]
    /// Handles changes to the requested canvas view size.
    pub fn update_size(&mut self, render_state: &RenderState, requested_dimensions: DimensionsPx) {
        let new_dimensions = requested_dimensions.to_nonzero();
        if new_dimensions == self.dimensions {
            return;
        }

        let device = &render_state.device;
        self.texture_view = Self::create_canvas_view_texture(device, new_dimensions);

        render_state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                device,
                &self.texture_view,
                wgpu::FilterMode::Linear,
                self.texture_id,
            );

        self.dimensions = new_dimensions;
    }

    /// Render the canvas view texture using the canvas texture with the camera uniform.
    ///
    /// Note: Don't call this function directly, call the wrapper function in `CanvasGpu` instead.
    pub(super) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        composite_canvas: &mut CompositeCanvas,
        layers: &mut Layers,
        enabled_layers_config: &mut EnabledLayersConfig,
    ) {
        composite_canvas.render_if_necessary(device, queue, layers, enabled_layers_config);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("canvas_view_render_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas_view_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
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

        queue.submit(iter::once(encoder.finish()));
    }

    /// Utility function to create a texture for the canvas view.
    fn create_canvas_view_texture(
        device: &wgpu::Device,
        texture_dimensions: DimensionsPxNonzero,
    ) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas_view_texture"),
            size: wgpu::Extent3d {
                width: texture_dimensions.width(),
                height: texture_dimensions.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}
