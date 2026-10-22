//! This module is responsible for managing complex interactions with the GPU. It is not
//! responsible for window or frame management.

mod camera;
mod canvas_view;
mod composite_canvas;
mod layers;

use crate::dimensions::DimensionsPxNonzero;
use camera::Camera;
use canvas_view::CanvasView;
use composite_canvas::CompositeCanvas;
use egui_wgpu::RenderState;
use layers::Layers;

/// All GPU resources that are tied to the canvas textures and rendering pipeline.
pub struct CanvasGpu {
    camera: Camera,
    layers: Layers,
    composite_canvas: CompositeCanvas,
    canvas_view: CanvasView,
}

impl CanvasGpu {
    #[must_use]
    pub fn new(
        render_state: &RenderState,
        canvas_dimensions: DimensionsPxNonzero,
        canvas_view_dimensions: DimensionsPxNonzero,
        initial_pixels: &[u8],
    ) -> Self {
        let device = &render_state.device;
        let queue = &render_state.queue;

        let camera = Camera::new(device);

        let mut layers = Layers::new(device);
        layers.add_layer(device, queue, canvas_dimensions, initial_pixels);

        let composite_canvas = CompositeCanvas::new(device, &layers, canvas_dimensions);

        let canvas_view = CanvasView::new(
            render_state,
            &camera,
            &composite_canvas,
            canvas_dimensions,
            canvas_view_dimensions,
        );

        Self {
            camera,
            layers,
            composite_canvas,
            canvas_view,
        }
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn layers(&mut self) -> &mut Layers {
        &mut self.layers
    }

    pub fn canvas_view(&mut self) -> &mut CanvasView {
        &mut self.canvas_view
    }

    pub fn render_canvas_view(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.canvas_view
            .render(device, queue, &mut self.composite_canvas, &mut self.layers);
    }

    /// Wrapper function for `queue.write_texture`.
    fn write_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        texture_dimensions: DimensionsPxNonzero,
        bytes: &[u8],
    ) {
        let width = texture_dimensions.width();
        let height = texture_dimensions.height();

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
