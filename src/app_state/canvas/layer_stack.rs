use crate::app_state::canvas::layer::{Layer, LayerId, TileCoord};
use crate::app_state::canvas::{LayerInitialPixels, MoveLayerDirection};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui_dnd::DragDropResponse;
use std::collections::HashMap;
use std::slice::IterMut;
use tiny_skia::{Color, Pixmap};

pub struct LayerStack {
    dimensions: DimensionsPxNonzero,

    // The direction of the layers is inverted from what you might expect, with index 0 being the
    // top layer and going down from there as the indices increase. This is necessary to allow
    // egui_dnd to reorder the vector itself without having to invert and de-invert it all the time,
    // since that library doesn't allow configuring the display order.
    layers: Vec<Layer>,
    selected_id: Option<LayerId>,
    next_id: LayerId,

    pub changed_since_last_save: bool,
}

impl LayerStack {
    pub fn new(canvas_gpu: &mut CanvasGpu, dimensions: DimensionsPxNonzero) -> Self {
        Self {
            dimensions,
            layers: vec![Layer::new(
                canvas_gpu,
                dimensions,
                "Background".to_owned(),
                LayerId(0),
                Color::WHITE,
            )],
            selected_id: Some(LayerId(0)),
            next_id: LayerId(1),
            changed_since_last_save: false,
        }
    }

    pub fn dimensions(&self) -> DimensionsPxNonzero {
        self.dimensions
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn selected_id(&self) -> &Option<LayerId> {
        &self.selected_id
    }

    pub fn layers_iter_mut(&mut self) -> IterMut<'_, Layer> {
        self.layers.iter_mut()
    }

    #[allow(dead_code)]
    pub fn get_layer(&self, layer_id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|layer| *layer.id() == layer_id)
    }

    pub fn get_layer_with_idx(&self, layer_id: LayerId) -> Option<(usize, &Layer)> {
        self.layers
            .iter()
            .enumerate()
            .find(|(_, layer)| *layer.id() == layer_id)
    }

    pub fn selected_layer_with_idx(&self) -> Option<(usize, &Layer)> {
        self.selected_id
            .and_then(|selected_layer_id| self.get_layer_with_idx(selected_layer_id))
    }

    pub fn selected_layer_mut(&mut self) -> Option<&mut Layer> {
        self.selected_id.and_then(|selected_layer_id| {
            self.layers
                .iter_mut()
                .find(|layer| *layer.id() == selected_layer_id)
        })
    }

    pub(super) fn resize(
        &mut self,
        canvas_gpu: &mut CanvasGpu,
        new_dimensions: DimensionsPxNonzero,
    ) {
        self.changed_since_last_save = true;
        self.dimensions = new_dimensions;

        for layer in &mut self.layers {
            layer.resize(canvas_gpu, new_dimensions);
        }
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
        layer_id: LayerId,
    ) -> Result<(), String> {
        self.changed_since_last_save = true;

        let (existing_index, existing_layer) = self
            .get_layer_with_idx(layer_id)
            .ok_or_else(|| format!("No layer with id {layer_id:?}"))?;

        // Clone all tiles from the existing layer.
        let cloned_tiles: HashMap<TileCoord, Pixmap> = existing_layer
            .tiles()
            .iter()
            .map(|(coord, pixmap)| (*coord, pixmap.clone()))
            .collect();

        self.add_layer_specified(
            canvas_gpu,
            LayerInitialPixels::Tiles(cloned_tiles, existing_layer.default_color),
            existing_index,
            existing_layer.enabled,
        );
        Ok(())
    }

    /// Adds a layer with specifiable properties.
    pub fn add_layer_specified(
        &mut self,
        canvas_gpu: &mut CanvasGpu,
        layer_initial_pixels: LayerInitialPixels,
        insert_idx: usize,
        enabled: bool,
    ) {
        self.changed_since_last_save = true;

        let insert_idx_checked = if insert_idx > self.layers.len() {
            self.layers.len()
        } else {
            insert_idx
        };

        let name = format!("Layer {}", self.next_id.0);
        let mut new_layer = match layer_initial_pixels {
            LayerInitialPixels::SolidColor(color) => {
                Layer::new(canvas_gpu, self.dimensions, name, self.next_id, color)
            }
            LayerInitialPixels::Tiles(tiles, uninitialized_color) => Layer::new_from_tiles(
                canvas_gpu,
                self.dimensions,
                name,
                self.next_id,
                tiles,
                uninitialized_color,
            ),
        };
        new_layer.enabled = enabled;
        self.next_id.0 += 1;

        self.selected_id = Some(*new_layer.id());

        self.layers.insert(insert_idx_checked, new_layer);
    }

    pub fn move_layer(
        &mut self,
        layer_id: LayerId,
        direction: &MoveLayerDirection,
    ) -> Result<(), String> {
        self.changed_since_last_save = true;

        let layer_idx = self
            .get_layer_with_idx(layer_id)
            .ok_or_else(|| format!("No layer with id {layer_id:?}"))?
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
        self.changed_since_last_save = true;

        response.update_vec(&mut self.layers);
    }

    pub fn merge_layer_to_adjacent(
        &mut self,
        layer_id: LayerId,
        direction: &MoveLayerDirection,
    ) -> Result<(), String> {
        let layer_idx = self
            .get_layer_with_idx(layer_id)
            .ok_or_else(|| format!("No layer with id {layer_id:?}"))?
            .0;

        match direction {
            MoveLayerDirection::Up => {
                if layer_idx > 0 {
                    // The layer array is inverted, so `layer_idx - 1` is the layer above.
                    let above_layer_idx = layer_idx - 1;
                    // Since merge_layer_down only supports merging down, we need to swap this
                    // layer with the one above it, then merge it down, so that things like layer
                    // names come from the correct layer.
                    self.layers.swap(layer_idx, above_layer_idx);
                    self.merge_layer_down(above_layer_idx);
                    Ok(())
                } else {
                    Err(String::from("Can't merge the top layer up"))
                }
            }
            MoveLayerDirection::Down => {
                if layer_idx < self.layers.len() - 1 {
                    self.merge_layer_down(layer_idx);
                    Ok(())
                } else {
                    Err(String::from("Can't merge the bottom layer down"))
                }
            }
        }
    }

    /// Merges the layer with the specified index down onto the one below it.
    fn merge_layer_down(&mut self, merge_layer_idx: usize) {
        if merge_layer_idx >= self.layers.len() - 1 {
            eprintln!(
                "Error: merge_layer_down called with invalid index \
                ({merge_layer_idx}, with array length {}). (Remember that 0 is the top.)",
                self.layers.len()
            );
            return;
        }

        self.changed_since_last_save = true;

        let merged_layer = self.layers.remove(merge_layer_idx);
        // Removing the merged layer shifts everything down in the array. The top layer has index 0,
        // so shifting down puts the layer below in place of the merged layer.
        let base_layer = self
            .layers
            .get_mut(merge_layer_idx)
            .expect("Function should be called with valid index");

        base_layer.composite_from_layer_above(&merged_layer);
        base_layer.default_color =
            Self::composite_colors(base_layer.default_color, merged_layer.default_color);

        let base_layer_id = *base_layer.id();
        self.set_selected_layer(base_layer_id);
    }

    pub fn delete_layer(&mut self, layer_id: LayerId) {
        self.changed_since_last_save = true;

        let selected_idx = self.selected_id.and_then(|selected_id| {
            self.layers
                .iter()
                .position(|layer| *layer.id() == selected_id)
        });

        if let Some(selected_idx) = selected_idx
            && *self.layers[selected_idx].id() == layer_id
        {
            self.layers.remove(selected_idx);
            self.selected_id = if self.layers.is_empty() {
                // If we just deleted the last layer, select nothing.
                None
            } else if selected_idx >= self.layers.len() {
                // If now past the end, shift up by one.
                Some(*self.layers[selected_idx - 1].id())
            } else {
                // Otherwise, keep the same index.
                Some(*self.layers[selected_idx].id())
            }
        } else {
            self.layers.retain(|layer| *layer.id() != layer_id);
        }
    }

    pub fn set_selected_layer(&mut self, layer_id: LayerId) {
        if self.layers.iter().any(|layer| *layer.id() == layer_id) {
            self.selected_id = Some(layer_id);
        } else {
            eprintln!("Error: Tried to select non-existent layer {layer_id:?}");
        }
    }

    fn composite_colors(below: Color, above: Color) -> Color {
        let below_a = below.alpha();
        let above_a = above.alpha();

        let out_a = below_a + above_a * (1.0 - below_a);
        if out_a <= 0.0 {
            return Color::TRANSPARENT;
        }

        let blend =
            |below: f32, above: f32| (above * above_a + below * below_a * (1.0 - above_a)) / out_a;

        Color::from_rgba(
            blend(below.red(), above.red()),
            blend(below.green(), above.green()),
            blend(below.blue(), above.blue()),
            out_a,
        )
        .expect("Outputs should be in 0..=1 because inputs are in 0..=1")
    }
}
