use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui::Rgba;
use egui_dnd::DragDropResponse;
use layer::Layer;
use std::slice::IterMut;
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

pub mod canvas_pos;
pub mod layer;

pub struct Canvas {
    dimensions: DimensionsPxNonzero,
    layers: Vec<Layer>,
    selected_layer_id: Option<i32>,
    next_layer_id: i32,
}

impl Canvas {
    pub fn new(dimensions: DimensionsPxNonzero) -> Canvas {
        Canvas {
            dimensions,
            layers: vec![Layer::new(dimensions, "Background".to_owned(), 0, true)],
            next_layer_id: 1,
            selected_layer_id: Some(0),
        }
    }

    pub fn dimensions(&self) -> DimensionsPxNonzero {
        self.dimensions
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    // The number of items in the array needs to be kept in sync with the GPU, but it's safe to
    // make the layers themselves mutable.
    pub fn layers_iter_mut(&mut self) -> IterMut<'_, Layer> {
        self.layers.iter_mut()
    }

    pub fn get_selected_layer(&mut self) -> Option<&mut Layer> {
        self.selected_layer_id.and_then(|selected_layer_id| {
            self.layers
                .iter_mut()
                .find(|layer| *layer.id() == selected_layer_id)
        })
    }

    // This allows us to safely update the layer order without cloning anything. It's safe because
    // `update_vec` only rearranges items within the Vec, never adding or removing items.
    pub fn update_layers_from_dnd_response(&mut self, response: &DragDropResponse) {
        response.update_vec(&mut self.layers);
    }

    pub fn add_layer(&mut self, canvas_gpu: &mut CanvasGpu) {
        let new_layer = Layer::new(
            self.dimensions,
            format!("Layer {}", self.next_layer_id),
            self.next_layer_id,
            false,
        );
        self.next_layer_id += 1;

        self.selected_layer_id = Some(*new_layer.id());

        canvas_gpu.layers().add_layer(
            self.dimensions,
            *new_layer.id(),
            new_layer.buffer_as_bytes(),
        );
        self.layers.insert(0, new_layer);
    }

    pub fn delete_layer(&mut self, layer_id: i32, canvas_gpu: &mut CanvasGpu) {
        let selected_layer_idx = self.selected_layer_id.and_then(|selected_layer_id| {
            self.layers
                .iter()
                .position(|layer| *layer.id() == selected_layer_id)
        });

        if let Some(selected_layer_idx) = selected_layer_idx
            && *self.layers[selected_layer_idx].id() == layer_id
        {
            self.layers.remove(selected_layer_idx);
            self.selected_layer_id = if self.layers.is_empty() {
                // If we just deleted the last layer, select nothing.
                None
            } else if selected_layer_idx >= self.layers.len() {
                // If now past the end, shift up by one.
                Some(*self.layers[selected_layer_idx - 1].id())
            } else {
                // Otherwise, keep the same index.
                Some(*self.layers[selected_layer_idx].id())
            }
        } else {
            self.layers.retain(|layer| *layer.id() != layer_id);
        }

        canvas_gpu.layers().delete_layer(layer_id);
    }

    pub fn set_selected_layer(&mut self, layer_id: i32) {
        if self.layers.iter().any(|layer| *layer.id() == layer_id) {
            self.selected_layer_id = Some(layer_id);
        } else {
            eprintln!("Error: Tried to select non-existent layer {layer_id}");
        }
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    #[allow(clippy::cast_sign_loss)]
    pub fn draw_line(&mut self, pos1: CanvasPos, pos2: CanvasPos, thickness: f32, raw_color: Rgba) {
        let Some(selected_layer) = self.get_selected_layer() else {
            return;
        };

        // Build a path for the line segment (or a zero-length segment for a dot).
        let x1 = pos1.x as f32 + 0.5;
        let y1 = pos1.y as f32 + 0.5;
        let x2 = pos2.x as f32 + 0.5;
        let y2 = pos2.y as f32 + 0.5;

        let mut pb = PathBuilder::new();
        pb.move_to(x1, y1);
        pb.line_to(x2, y2);
        let Some(path) = pb.finish() else {
            return;
        };

        let stroke = Stroke {
            width: thickness,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        let converted_color: Color = Color::from_rgba(
            raw_color[0].clamp(0.0, 1.0),
            raw_color[1].clamp(0.0, 1.0),
            raw_color[2].clamp(0.0, 1.0),
            raw_color[3].clamp(0.0, 1.0),
        )
        .expect("Color values should be clamped properly");
        let mut paint = Paint::default();
        paint.set_color(converted_color);
        paint.anti_alias = true;

        selected_layer
            .pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        selected_layer.dirty = true;
    }
}
