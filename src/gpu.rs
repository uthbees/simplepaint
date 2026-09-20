//! This module handles interactions with the GPU that are too complex to fit into app.rs.

use crate::canvas::Canvas;
use egui_wgpu::RenderState;

/// All GPU resources that are tied to the canvas texture.
pub struct CanvasGpu {
    /// The wgpu texture for the canvas display.
    pub texture: wgpu::Texture,
    /// The canvas texture's id.
    pub texture_id: egui::TextureId,
}

impl CanvasGpu {
    #[must_use]
    /// Create the texture and register it with the egui renderer.
    pub fn new(render_state: &RenderState, width: u32, height: u32, pixels: &[u8]) -> Self {
        let device = &render_state.device;
        let queue = &render_state.queue;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas_texture"),
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

        CanvasGpu::write_texture(queue, &texture, width, height, pixels);

        let texture_id = render_state.renderer.write().register_native_texture(
            device,
            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::FilterMode::Nearest,
        );

        Self {
            texture,
            texture_id,
        }
    }

    /// Re-upload the full CPU buffer to the GPU texture.
    pub fn upload(&self, queue: &wgpu::Queue, canvas: &Canvas) {
        CanvasGpu::write_texture(
            queue,
            &self.texture,
            canvas.width(),
            canvas.height(),
            canvas.as_bytes(),
        );
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
