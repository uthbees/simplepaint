use crate::app_state::ToolSettings;
use crate::app_state::canvas::canvas_edge_masks::CanvasEdgeMasks;
use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::app_state::canvas::layer::{Layer, LayerId, TILE_SIZE, TileCoord};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

pub const STAGING_LAYER_ID: LayerId = LayerId(-1);

/// A reserved layer for holding in-progress draw actions.
pub struct StagingLayer {
    dimensions: DimensionsPxNonzero,
    layer: Layer,
}

impl StagingLayer {
    pub fn new(canvas_gpu: &mut CanvasGpu, dimensions: DimensionsPxNonzero) -> Self {
        Self {
            dimensions,
            layer: Layer::new(
                canvas_gpu,
                dimensions,
                "Staging Layer".to_owned(),
                STAGING_LAYER_ID,
                Color::TRANSPARENT,
            ),
        }
    }

    #[allow(dead_code)]
    pub fn layer(&self) -> &Layer {
        &self.layer
    }

    pub fn layer_mut(&mut self) -> &mut Layer {
        &mut self.layer
    }

    pub fn resize(&mut self, canvas_gpu: &mut CanvasGpu, new_dimensions: DimensionsPxNonzero) {
        self.dimensions = new_dimensions;
        self.layer.resize(canvas_gpu, new_dimensions);
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

        let mut masks = CanvasEdgeMasks::new(self.dimensions);

        // Rasterize the stroke onto each affected tile individually.
        for tile_y in tile_min_y..=tile_max_y {
            for tile_x in tile_min_x..=tile_max_x {
                let Some(tile) = self.layer.get_tile_mut(TileCoord {
                    x: tile_x,
                    y: tile_y,
                }) else {
                    continue;
                };

                // Translate from canvas coordinates to tile coordinates: the tile's world origin is
                // (tx * TILE_SIZE, ty * TILE_SIZE), so we shift the path by the negative of that.
                let tile_world_origin = CanvasPos {
                    x: tile_x * tile_size,
                    y: tile_y * tile_size,
                };
                let transform = Transform::from_translate(
                    -tile_world_origin.x as f32,
                    -tile_world_origin.y as f32,
                );

                let first_invalid_canvas_x = self.dimensions.width().cast_signed();
                let first_invalid_canvas_y = self.dimensions.height().cast_signed();

                let on_right_edge = (tile_world_origin.x + tile_size - 1) >= first_invalid_canvas_x;
                let on_bottom_edge =
                    (tile_world_origin.y + tile_size - 1) >= first_invalid_canvas_y;

                let mask = if on_right_edge && on_bottom_edge {
                    Some(masks.bottom_right())
                } else if on_right_edge {
                    Some(masks.right())
                } else if on_bottom_edge {
                    Some(masks.bottom())
                } else {
                    None
                };

                tile.stroke_path(&path, &paint, &stroke, transform, mask);
            }
        }
    }
}
