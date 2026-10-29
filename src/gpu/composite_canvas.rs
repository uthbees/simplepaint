use crate::gpu::layers::Layers;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui_wgpu::RenderState;
use std::iter;
use wgpu::PipelineCompilationOptions;

/// The composite canvas is the texture containing the canvas data
/// with all layers composited together.
pub struct CompositeCanvas {
    render_state: RenderState,
    pipeline: wgpu::RenderPipeline,
    texture_view: wgpu::TextureView,
    last_draw_layers_command: Option<DrawLayersCommand>,
}

/// An array of layer ids specifying which layers to draw and in what order.
#[derive(PartialEq)]
pub struct DrawLayersCommand(pub Vec<i32>);

impl CompositeCanvas {
    pub fn new(
        render_state: RenderState,
        layers: &Layers,
        canvas_dimensions: DimensionsPxNonzero,
    ) -> Self {
        let device = &render_state.device;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("composite_canvas_texture"),
            size: wgpu::Extent3d {
                width: canvas_dimensions.width(),
                height: canvas_dimensions.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composite_canvas_pipeline_layout"),
            bind_group_layouts: &[Some(layers.composition_bind_group_layout())],
            immediate_size: 0,
        });
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/composite_canvas.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite_canvas_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture.format(),
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

        Self {
            render_state,
            pipeline,
            texture_view,
            last_draw_layers_command: None,
        }
    }

    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    /// Render the composite canvas texture from the layer textures if any layer has been updated
    /// since the last composite canvas render.
    pub fn render_if_necessary(
        &mut self,
        layers: &mut Layers,
        draw_layers_command: DrawLayersCommand,
    ) {
        if !layers.dirty && self.last_draw_layers_command.as_ref() == Some(&draw_layers_command) {
            return;
        }

        let mut encoder =
            self.render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("composite_canvas_render_encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite_canvas_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
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

            render_pass.set_pipeline(&self.pipeline);

            let layer_map = layers.layer_map();
            for layer_id in &draw_layers_command.0 {
                if let Some(layer) = layer_map.get(layer_id) {
                    render_pass.set_bind_group(0, &layer.composition_bind_group, &[]);
                    render_pass.draw(0..3, 0..1);
                } else {
                    eprintln!(
                        "GPU-side layer {} not found, skipping rendering for layer.",
                        layer_id
                    );
                }
            }
        }

        self.render_state.queue.submit(iter::once(encoder.finish()));
        layers.dirty = false;
        self.last_draw_layers_command = Some(draw_layers_command);
    }
}
