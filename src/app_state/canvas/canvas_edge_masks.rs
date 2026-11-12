use crate::app_state::canvas::layer::TILE_SIZE;
use crate::utils::dimensions::DimensionsPxNonzero;
use tiny_skia::Mask;

/// Stores canvas edge masks for drawing on tiles on the edge of the canvas without drawing off
/// the canvas.
///
/// Initializes each mask on demand.
pub struct CanvasEdgeMasks {
    canvas_dimensions: DimensionsPxNonzero,

    right: Option<Mask>,
    bottom: Option<Mask>,
    bottom_right: Option<Mask>,
}

impl CanvasEdgeMasks {
    pub fn new(canvas_dimensions: DimensionsPxNonzero) -> Self {
        Self {
            canvas_dimensions,
            right: None,
            bottom: None,
            bottom_right: None,
        }
    }

    pub fn right(&mut self) -> &Mask {
        self.right.get_or_insert_with(|| {
            Self::build_mask(self.canvas_dimensions.width() % TILE_SIZE, TILE_SIZE)
        })
    }

    pub fn bottom(&mut self) -> &Mask {
        self.bottom.get_or_insert_with(|| {
            Self::build_mask(TILE_SIZE, self.canvas_dimensions.height() % TILE_SIZE)
        })
    }

    pub fn bottom_right(&mut self) -> &Mask {
        self.bottom_right.get_or_insert_with(|| {
            Self::build_mask(
                self.canvas_dimensions.width() % TILE_SIZE,
                self.canvas_dimensions.height() % TILE_SIZE,
            )
        })
    }

    /// Builds a mask disallowing the right and bottom edges based on specified coordinates
    /// within the tile.
    ///
    /// To fully include the x or y dimension, pass `TILE_SIZE` (which is one larger than the
    /// maximum coordinate within a tile).
    fn build_mask(first_invalid_x: u32, first_invalid_y: u32) -> Mask {
        let first_invalid_x = first_invalid_x.min(TILE_SIZE) as usize;
        let first_invalid_y = first_invalid_y.min(TILE_SIZE) as usize;

        let mut mask =
            Mask::new(TILE_SIZE, TILE_SIZE).expect("Tile size should be valid for a mask");
        let data = mask.data_mut();

        // The mask starts off with everything excluded (black), so just mark everything that
        // should be included as white.
        for y in 0..first_invalid_y {
            let row_start = y * TILE_SIZE as usize;
            data[row_start..row_start + first_invalid_x].fill(255);
        }

        mask
    }
}
