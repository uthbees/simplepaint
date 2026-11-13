use crate::gpu::layers::Layers;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui_wgpu::RenderState;
use image::ImageBuffer;
use std::iter;
use std::path::Path;
use wgpu::PipelineCompilationOptions;

/// The composite canvas is the texture containing the canvas data
/// with all layers composited together.
pub struct CompositeCanvas {
    render_state: RenderState,
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
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

        let (texture, texture_view) = Self::create_texture(&render_state, canvas_dimensions);

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
            texture,
            texture_view,
            last_draw_layers_command: None,
        }
    }

    fn create_texture(
        render_state: &RenderState,
        dimensions: DimensionsPxNonzero,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = render_state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("composite_canvas_texture"),
                size: wgpu::Extent3d {
                    width: dimensions.width(),
                    height: dimensions.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, texture_view)
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
                    eprintln!("GPU-side layer {layer_id} not found, skipping rendering for layer.");
                }
            }
        }

        self.render_state.queue.submit(iter::once(encoder.finish()));
        layers.dirty = false;
        self.last_draw_layers_command = Some(draw_layers_command);
    }

    /// Invalidates the canvas view bind group - should only be used by `CanvasGpu`.
    pub(super) fn resize(&mut self, new_dimensions: DimensionsPxNonzero) {
        let (texture, texture_view) = Self::create_texture(&self.render_state, new_dimensions);
        self.texture = texture;
        self.texture_view = texture_view;
    }

    pub fn save_texture_to_file(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let device = &self.render_state.device;
        let width = self.texture.width();
        let height = self.texture.height();

        // Calculate the buffer size including the row padding required by wgpu.
        let bytes_per_pixel = 4; // RGBA8
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) & !(align - 1);
        let buffer_size = u64::from(padded_bytes_per_row * height);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite_canvas_save_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("composite_canvas_save_encoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.render_state.queue.submit(Some(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(err) = sender.send(result) {
                eprintln!("GPU error saving texture to file: {err}");
            }
        });

        device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;

        let data = buffer_slice.get_mapped_range()?;
        let mut unpadded_data = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);

        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            unpadded_data.extend_from_slice(&data[start..end]);
        }

        drop(data);
        buffer.unmap();

        let img: ImageBuffer<image::Rgba<u8>, _> =
            ImageBuffer::from_raw(width, height, unpadded_data)
                .ok_or("Failed to create image buffer from raw texture data")?;

        img.save(file_path)?;

        Ok(())
    }
}
