//! This module is responsible for managing complex interactions with the GPU. It is not
//! responsible for window or frame management.

pub mod camera;
pub mod canvas_view;
pub mod composite_canvas;
pub mod layers;

use crate::app_state::canvas::Canvas;
use crate::app_state::canvas::staging_layer::STAGING_LAYER_ID;
use crate::gpu::composite_canvas::DrawLayersCommand;
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
    ) -> Self {
        let camera = Camera::new(render_state.clone());

        let layers = Layers::new(render_state.clone());

        let composite_canvas =
            CompositeCanvas::new(render_state.clone(), &layers, canvas_dimensions);

        let canvas_view = CanvasView::new(
            render_state,
            &camera,
            &composite_canvas,
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

    pub fn composite_canvas(&mut self) -> &mut CompositeCanvas {
        &mut self.composite_canvas
    }

    pub fn canvas_view(&mut self) -> &mut CanvasView {
        &mut self.canvas_view
    }

    pub fn render_canvas_view(&mut self, canvas: &Canvas) {
        let selected_layer_id = canvas.layer_stack().selected_id();
        let mut draw_layers_command = DrawLayersCommand(vec![]);

        // Reverse the layer order so that we draw from the bottom up.
        for cpu_layer in canvas.layer_stack().layers().iter().rev() {
            if cpu_layer.enabled {
                draw_layers_command.0.push(*cpu_layer.id());

                if let Some(selected_layer_id) = selected_layer_id
                    && *selected_layer_id == *cpu_layer.id()
                {
                    draw_layers_command.0.push(STAGING_LAYER_ID);
                }
            }
        }

        self.canvas_view.render(
            &mut self.composite_canvas,
            &mut self.layers,
            draw_layers_command,
        );
    }

    pub fn resize_composite_canvas(&mut self, new_canvas_dimensions: DimensionsPxNonzero) {
        self.composite_canvas.resize(new_canvas_dimensions);
        self.canvas_view
            .update_bind_group(&self.camera, &self.composite_canvas);
    }
}
