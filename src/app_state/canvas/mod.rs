use crate::app_state::canvas::layer::TileCoord;
use crate::app_state::canvas::staging_layer::StagingLayer;
use crate::app_state::{MAX_CANVAS_HEIGHT, MAX_CANVAS_WIDTH};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use layer_stack::LayerStack;
use std::collections::HashMap;
use tiny_skia::{Color, Pixmap};

mod canvas_edge_masks;
pub mod canvas_pos;
pub mod layer;
pub mod layer_stack;
pub mod staging_layer;

pub struct Canvas {
    staging_layer: StagingLayer,
    layer_stack: LayerStack,
}

impl Canvas {
    pub fn new(canvas_gpu: &mut CanvasGpu, dimensions: DimensionsPxNonzero) -> Canvas {
        let dimensions = dimensions.clamp_max_values(MAX_CANVAS_WIDTH, MAX_CANVAS_HEIGHT);

        Canvas {
            staging_layer: StagingLayer::new(canvas_gpu, dimensions),
            layer_stack: LayerStack::new(canvas_gpu, dimensions),
        }
    }

    pub fn dimensions(&self) -> DimensionsPxNonzero {
        self.layer_stack.dimensions()
    }

    pub fn layer_stack(&self) -> &LayerStack {
        &self.layer_stack
    }

    pub fn layer_stack_mut(&mut self) -> &mut LayerStack {
        &mut self.layer_stack
    }

    #[allow(dead_code)]
    pub fn staging_layer(&self) -> &StagingLayer {
        &self.staging_layer
    }

    pub fn staging_layer_mut(&mut self) -> &mut StagingLayer {
        &mut self.staging_layer
    }

    pub fn resize(&mut self, canvas_gpu: &mut CanvasGpu, new_dimensions: DimensionsPxNonzero) {
        let new_dimensions = new_dimensions.clamp_max_values(MAX_CANVAS_WIDTH, MAX_CANVAS_HEIGHT);

        self.staging_layer.resize(canvas_gpu, new_dimensions);
        self.layer_stack.resize(canvas_gpu, new_dimensions);
        canvas_gpu.resize_composite_canvas(new_dimensions);
    }

    pub fn commit_staging_layer(&mut self) {
        if let Some(selected_layer) = self.layer_stack.selected_layer_mut() {
            selected_layer.composite_from_layer_above(self.staging_layer.layer());
        }

        self.staging_layer.layer_mut().clear();
    }
}

pub enum MoveLayerDirection {
    Up,
    Down,
}

pub enum LayerInitialPixels {
    SolidColor(Color),
    Tiles(HashMap<TileCoord, Pixmap>, Color),
}
