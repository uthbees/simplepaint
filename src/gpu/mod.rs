//! This module is responsible for managing complex interactions with the GPU. It is not
//! responsible for window or frame management.

pub mod camera;
pub mod canvas_view;
pub mod composite_canvas;
pub mod layers;

use crate::app_state::canvas::layer::Layer;
use crate::utils::dimensions::DimensionsPxNonzero;
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
        render_state: RenderState,
        canvas_dimensions: DimensionsPxNonzero,
        canvas_view_dimensions: DimensionsPxNonzero,
        initial_layer_id: i32,
        initial_pixels: &[u8],
    ) -> Self {
        let camera = Camera::new(render_state.clone());

        let mut layers = Layers::new(render_state.clone());
        layers.add_layer(canvas_dimensions, initial_layer_id, initial_pixels);

        let composite_canvas =
            CompositeCanvas::new(render_state.clone(), &layers, canvas_dimensions);

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

    pub fn render_canvas_view(&mut self, cpu_layers: &[Layer]) {
        self.canvas_view
            .render(&mut self.composite_canvas, &mut self.layers, cpu_layers);
    }
}
