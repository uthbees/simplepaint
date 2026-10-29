use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui::Rgba;
use egui_dnd::DragDropResponse;
use layer::Layer;
use std::slice::IterMut;
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

pub mod canvas_pos;
pub mod layer;

// layers: vec![
//     1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 21, 31, 41, 51, 61, 71, 81, 91, 101, 12, 22,
//     32, 42, 52, 62, 72, 82, 92, 102, 13, 23, 33, 43, 53, 63, 73, 83, 93, 103, 14,
//     24, 34, 44, 54, 64, 74, 84, 94, 104,
// ],

pub struct Canvas {
    dimensions: DimensionsPxNonzero,
    layers: Vec<Layer>,

    selected_layer_idx: Option<usize>,
}

impl Canvas {
    pub fn new(dimensions: DimensionsPxNonzero) -> Canvas {
        Canvas {
            dimensions,
            layers: vec![Layer::new(dimensions, "Background".to_owned(), 0, true)],
            selected_layer_idx: Some(0),
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

    // This allows us to safely update the layer order without cloning anything. It's safe because
    // `update_vec` only rearranges items within the Vec, never adding or removing items.
    pub fn update_layers_from_dnd_response(&mut self, response: &DragDropResponse) {
        response.update_vec(&mut self.layers);
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    #[allow(clippy::cast_sign_loss)]
    pub fn draw_line(&mut self, pos1: CanvasPos, pos2: CanvasPos, thickness: f32, raw_color: Rgba) {
        let Some(active_layer) = self.get_active_layer() else {
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

        active_layer
            .pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        active_layer.dirty = true;
    }

    fn get_active_layer(&mut self) -> Option<&mut Layer> {
        let selected_layer_idx = self.selected_layer_idx?;
        assert!(selected_layer_idx < self.layers.len());
        Some(&mut self.layers[selected_layer_idx])
    }
}
