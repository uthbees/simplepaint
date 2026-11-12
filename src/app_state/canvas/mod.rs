use crate::app_state::ToolSettings;
use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::app_state::canvas::layer::{Layer, TILE_SIZE, TileCoord};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui_dnd::DragDropResponse;
use std::collections::HashMap;
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
                Color::WHITE,
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
            LayerInitialPixels::SolidColor(Color::TRANSPARENT),
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

        // Clone all tiles from the existing layer.
        let cloned_tiles: HashMap<TileCoord, Pixmap> = existing_layer
            .tiles()
            .iter()
            .map(|(coord, pixmap)| (*coord, pixmap.clone()))
            .collect();

        self.add_layer_specified(
            canvas_gpu,
            LayerInitialPixels::Tiles(cloned_tiles, existing_layer.default_color()),
            existing_index,
            existing_layer.enabled,
        );
        Ok(())
    }

    /// Adds a layer with specifiable properties.
    fn add_layer_specified(
        &mut self,
        canvas_gpu: &mut CanvasGpu,
        layer_initial_pixels: LayerInitialPixels,
        insert_idx: usize,
        enabled: bool,
    ) {
        let insert_idx_checked = if insert_idx > self.layers.len() {
            self.layers.len()
        } else {
            insert_idx
        };

        let name = format!("Layer {}", self.next_layer_id);
        let mut new_layer = match layer_initial_pixels {
            LayerInitialPixels::SolidColor(color) => {
                Layer::new(self.dimensions, name, self.next_layer_id, color)
            }
            LayerInitialPixels::Tiles(tiles, uninitialized_color) => Layer::new_from_tiles(
                self.dimensions,
                name,
                self.next_layer_id,
                tiles,
                uninitialized_color,
            ),
        };
        new_layer.enabled = enabled;
        self.next_layer_id += 1;

        self.selected_layer_id = Some(*new_layer.id());

        canvas_gpu.layers().add_layer(
            self.dimensions,
            *new_layer.id(),
            &new_layer.buffer_as_bytes(),
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

        // For every tile in the merged layer, draw it onto the corresponding tile
        // in the kept layer (creating it if needed).
        for (coord, merged_tile) in merged_layer.tiles() {
            if let Some(dest_tile) = kept_layer.get_tile_mut(*coord) {
                dest_tile.as_mut().draw_pixmap(
                    0,
                    0,
                    merged_tile.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }
        }
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
    ///
    /// The stroke is run with offset on each tile in the bounding box defined by `pos1` and `pos2`
    /// so that only tiles that are modified or nearby are marked as dirty.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    pub fn draw_line(
        &mut self,
        pos1: CanvasPos,
        pos2: CanvasPos,
        tool_settings: &ToolSettings,
        use_bg_color: bool,
    ) {
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
            width: tool_settings.stroke_width as f32,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        let raw_color = if use_bg_color {
            tool_settings.background_color
        } else {
            tool_settings.foreground_color
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
        paint.anti_alias = tool_settings.anti_alias;

        // Determine the bounding box of affected tiles.
        let half_thickness = (tool_settings.stroke_width as f32 * 0.5).ceil() as i32 + 1;
        let min_x = pos1.x.min(pos2.x) - half_thickness;
        let min_y = pos1.y.min(pos2.y) - half_thickness;
        let max_x = pos1.x.max(pos2.x) + half_thickness;
        let max_y = pos1.y.max(pos2.y) + half_thickness;

        let tile_size = TILE_SIZE.cast_signed();
        let tile_min_x = min_x.div_euclid(tile_size);
        let tile_min_y = min_y.div_euclid(tile_size);
        let tile_max_x = max_x.div_euclid(tile_size);
        let tile_max_y = max_y.div_euclid(tile_size);

        // Rasterize the stroke onto each affected tile individually.
        for tile_y in tile_min_y..=tile_max_y {
            for tile_x in tile_min_x..=tile_max_x {
                let Some(tile) = selected_layer.get_tile_mut(TileCoord {
                    x: tile_x,
                    y: tile_y,
                }) else {
                    continue;
                };

                // Translate from canvas coordinates to tile coordinates: the tile's world origin is
                // (tx * TILE_SIZE, ty * TILE_SIZE), so we shift the path by the negative of that.
                let offset_x = -(tile_x as f32 * TILE_SIZE as f32);
                let offset_y = -(tile_y as f32 * TILE_SIZE as f32);
                let transform = Transform::from_translate(offset_x, offset_y);

                tile.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }
    }
}

pub enum MoveLayerDirection {
    Up,
    Down,
}

enum LayerInitialPixels {
    SolidColor(Color),
    Tiles(HashMap<TileCoord, Pixmap>, Color),
}
