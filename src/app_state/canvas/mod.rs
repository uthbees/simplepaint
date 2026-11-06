use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::app_state::canvas::layer::InitialColor;
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui::Rgba;
use egui_dnd::DragDropResponse;
use layer::Layer;
use std::slice::IterMut;
use tiny_skia::{
    Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform,
};

pub mod canvas_pos;
pub mod layer;

pub struct Canvas {
    dimensions: DimensionsPxNonzero,
    // The direction of the layers is inverted from what you might expect, with index 0 being the
    // top layer and going down from there as the indices increase. This is necessary to allow
    // egui_dnd to reorder the vector itself without having to invert and de-invert it all the time,
    // since that library doesn't allow configuring the display order.
    layers: Vec<Layer>,
    selected_layer_id: Option<i32>,
    next_layer_id: i32,
}

impl Canvas {
    pub fn new(dimensions: DimensionsPxNonzero) -> Canvas {
        Canvas {
            dimensions,
            layers: vec![Layer::new(
                dimensions,
                "Background".to_owned(),
                0,
                &InitialColor::White,
            )],
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

    #[allow(dead_code)]
    pub fn get_layer(&self, layer_id: i32) -> Option<&Layer> {
        self.layers.iter().find(|layer| *layer.id() == layer_id)
    }

    pub fn get_layer_with_idx(&self, layer_id: i32) -> Option<(usize, &Layer)> {
        self.layers
            .iter()
            .enumerate()
            .find(|(_, layer)| *layer.id() == layer_id)
    }

    pub fn selected_layer_with_idx(&self) -> Option<(usize, &Layer)> {
        self.selected_layer_id
            .and_then(|selected_layer_id| self.get_layer_with_idx(selected_layer_id))
    }

    pub fn selected_layer_mut(&mut self) -> Option<&mut Layer> {
        self.selected_layer_id.and_then(|selected_layer_id| {
            self.layers
                .iter_mut()
                .find(|layer| *layer.id() == selected_layer_id)
        })
    }

    pub fn add_layer(&mut self, canvas_gpu: &mut CanvasGpu) {
        self.add_layer_specified(
            canvas_gpu,
            LayerInitialBuffer::SolidColor(InitialColor::Transparent),
            0,
            true,
        );
    }

    pub fn duplicate_layer(
        &mut self,
        canvas_gpu: &mut CanvasGpu,
        layer_id: i32,
    ) -> Result<(), String> {
        let (existing_index, existing_layer) = self
            .get_layer_with_idx(layer_id)
            .ok_or_else(|| format!("No layer with id {layer_id}"))?;

        self.add_layer_specified(
            canvas_gpu,
            LayerInitialBuffer::Pixmap(existing_layer.pixmap.clone()),
            existing_index,
            existing_layer.enabled,
        );
        Ok(())
    }

    /// Adds a layer with specifiable properties.
    ///
    /// If using `LayerInitialBuffer::Pixmap`, the pixmap dimensions must be identical to self.dimensions.
    fn add_layer_specified(
        &mut self,
        canvas_gpu: &mut CanvasGpu,
        layer_initial_buffer: LayerInitialBuffer,
        insert_idx: usize,
        enabled: bool,
    ) {
        let insert_idx_checked = if insert_idx > self.layers.len() {
            self.layers.len()
        } else {
            insert_idx
        };

        let name = format!("Layer {}", self.next_layer_id);
        let mut new_layer = match layer_initial_buffer {
            LayerInitialBuffer::SolidColor(color) => {
                Layer::new(self.dimensions, name, self.next_layer_id, &color)
            }
            LayerInitialBuffer::Pixmap(pixmap) => {
                assert_eq!(self.dimensions.width(), pixmap.width());
                assert_eq!(self.dimensions.height(), pixmap.height());
                Layer::new_from_pixmap(name, self.next_layer_id, pixmap)
            }
        };
        new_layer.enabled = enabled;
        self.next_layer_id += 1;

        self.selected_layer_id = Some(*new_layer.id());

        canvas_gpu.layers().add_layer(
            self.dimensions,
            *new_layer.id(),
            new_layer.buffer_as_bytes(),
        );
        self.layers.insert(insert_idx_checked, new_layer);
    }

    pub fn move_layer(
        &mut self,
        layer_id: i32,
        direction: &MoveLayerDirection,
    ) -> Result<(), String> {
        let layer_idx = self
            .get_layer_with_idx(layer_id)
            .ok_or_else(|| format!("No layer with id {layer_id}"))?
            .0;

        match direction {
            MoveLayerDirection::Up => {
                if layer_idx > 0 {
                    self.layers.swap(layer_idx, layer_idx - 1);
                    Ok(())
                } else {
                    Err(String::from("Can't move the top layer any farther up"))
                }
            }
            MoveLayerDirection::Down => {
                if layer_idx < self.layers.len() - 1 {
                    self.layers.swap(layer_idx, layer_idx + 1);
                    Ok(())
                } else {
                    Err(String::from("Can't move the bottom layer any farther down"))
                }
            }
        }
    }

    // This allows us to safely update the layer order without cloning anything. It's safe because
    // `update_vec` only rearranges items within the Vec, never adding or removing items.
    pub fn update_layers_from_dnd_response(&mut self, response: &DragDropResponse) {
        response.update_vec(&mut self.layers);
    }

    pub fn merge_layer_to_adjacent(
        &mut self,
        layer_id: i32,
        direction: &MoveLayerDirection,
    ) -> Result<(), String> {
        let layer_idx = self
            .get_layer_with_idx(layer_id)
            .ok_or_else(|| format!("No layer with id {layer_id}"))?
            .0;

        match direction {
            MoveLayerDirection::Up => {
                if layer_idx > 0 {
                    self.merge_adjacent_layers(layer_idx - 1, layer_idx);
                    Ok(())
                } else {
                    Err(String::from("Can't merge the top layer up"))
                }
            }
            MoveLayerDirection::Down => {
                if layer_idx < self.layers.len() - 1 {
                    self.merge_adjacent_layers(layer_idx + 1, layer_idx);
                    Ok(())
                } else {
                    Err(String::from("Can't merge the bottom layer down"))
                }
            }
        }
    }

    fn merge_adjacent_layers(&mut self, kept_layer_idx: usize, merged_layer_idx: usize) {
        assert_ne!(kept_layer_idx, merged_layer_idx);
        assert!(kept_layer_idx < self.layers.len());
        assert!(merged_layer_idx < self.layers.len());
        // I don't think it would be hard to support merging non-adjacent layers,
        // but there's no UI for it, so there's no reason to bother.
        assert_eq!(kept_layer_idx.abs_diff(merged_layer_idx), 1);

        let merged_layer = self.layers.remove(merged_layer_idx);

        let new_kept_layer_idx = if merged_layer_idx < kept_layer_idx {
            kept_layer_idx - 1
        } else {
            kept_layer_idx
        };

        let kept_layer = self
            .layers
            .get_mut(new_kept_layer_idx)
            .expect("Function should be called with valid index");

        kept_layer.pixmap.as_mut().draw_pixmap(
            0,
            0,
            merged_layer.pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        kept_layer.dirty = true;

        let kept_layer_id = *kept_layer.id();
        self.set_selected_layer(kept_layer_id);
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
        let Some(selected_layer) = self.selected_layer_mut() else {
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

pub enum MoveLayerDirection {
    Up,
    Down,
}

enum LayerInitialBuffer {
    SolidColor(InitialColor),
    Pixmap(Pixmap),
}
