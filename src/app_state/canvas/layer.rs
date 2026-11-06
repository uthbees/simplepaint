use crate::utils::dimensions::DimensionsPxNonzero;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use tiny_skia::{Color, Pixmap};

/// Side length of every square tile, in pixels.
pub const TILE_SIZE: u32 = 256;

/// Integer coordinate identifying a tile in the sparse grid.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

/// Stores and manages the state of a canvas layer.
///
/// Pixel data is stored in a sparse hash map of `TILE_SIZE × TILE_SIZE` tiles.  Tiles that have
/// never been painted are not allocated, and their color is determined by `default_color`.
#[derive(Debug)]
pub struct Layer {
    tiles: HashMap<TileCoord, Pixmap>,
    /// The color returned for any tile that has not yet been allocated.
    default_color: Color,
    /// Dimensions of the layer. Should always be identical to the dimensions of the canvas.
    dimensions: DimensionsPxNonzero,
    pub dirty: bool,
    dirty_tiles: HashSet<TileCoord>,

    id: i32,
    pub name: String,
    pub enabled: bool,
}

impl Layer {
    /// Creates a new layer of a solid color with no tiles allocated.
    pub fn new(dimensions: DimensionsPxNonzero, name: String, id: i32, color: Color) -> Self {
        Self {
            tiles: HashMap::new(),
            default_color: color,
            dimensions,
            dirty: false,
            dirty_tiles: HashSet::new(),
            id,
            name,
            enabled: true,
        }
    }

    /// Creates a new layer from a set of pre-existing tiles.
    ///
    /// Any tile coordinate not present in the map takes the color of `uninitialized_color`.
    pub fn new_from_tiles(
        dimensions: DimensionsPxNonzero,
        name: String,
        id: i32,
        tiles: HashMap<TileCoord, Pixmap>,
        uninitialized_color: Color,
    ) -> Self {
        Self {
            tiles,
            default_color: uninitialized_color,
            dimensions,
            dirty: false,
            dirty_tiles: HashSet::new(),
            id,
            name,
            enabled: true,
        }
    }

    pub fn id(&self) -> &i32 {
        &self.id
    }

    #[allow(dead_code)]
    pub fn dimensions(&self) -> DimensionsPxNonzero {
        self.dimensions
    }

    pub fn default_color(&self) -> Color {
        self.default_color
    }

    pub fn tiles(&self) -> &HashMap<TileCoord, Pixmap> {
        &self.tiles
    }

    /// Returns a mutable reference to the tile at `coord`, creating a new tile filled with the
    /// layer's `default_color` if one does not already exist. Marks the tile and layer as dirty.
    pub fn get_tile_mut(&mut self, coord: TileCoord) -> &mut Pixmap {
        self.dirty = true;
        self.dirty_tiles.insert(coord);
        self.tiles.entry(coord).or_insert_with(|| {
            let mut pm = Pixmap::new(TILE_SIZE, TILE_SIZE)
                .expect("Tile pixmap creation should succeed for TILE_SIZE > 0");
            pm.fill(self.default_color);
            pm
        })
    }

    pub fn dirty_tiles(&self) -> &HashSet<TileCoord> {
        &self.dirty_tiles
    }

    /// Clears the dirty tracking flags for both the layer itself and all its tiles.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
        self.dirty_tiles.clear();
    }

    /// Exports the entire layer as a flat premultiplied-RGBA byte buffer sized to
    /// `width × height × 4`.
    ///
    /// Unallocated tiles are filled with the layer's `default_color`.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    pub fn buffer_as_bytes(&self) -> Vec<u8> {
        let canvas_width = self.dimensions.width();
        let canvas_height = self.dimensions.height();
        let stride = (canvas_width * 4) as usize;
        let mut buffer = vec![0u8; (canvas_width * canvas_height * 4) as usize];

        // Pre-fill with the default color (only needed when it's not transparent, which is
        // already represented by the zeroed buffer).
        if self.default_color != Color::TRANSPARENT {
            let default_color_premultiplied = self.default_color.premultiply();
            let r = (default_color_premultiplied.red() * 255.0) as u8;
            let g = (default_color_premultiplied.green() * 255.0) as u8;
            let b = (default_color_premultiplied.blue() * 255.0) as u8;
            let a = (default_color_premultiplied.alpha() * 255.0) as u8;
            for pixel in buffer.as_chunks_mut::<4>().0 {
                pixel[0] = r;
                pixel[1] = g;
                pixel[2] = b;
                pixel[3] = a;
            }
        }

        // Blit each tile into the buffer.
        for (coord, tile) in &self.tiles {
            let tile_start_x = coord.x * TILE_SIZE.cast_signed();
            let tile_start_y = coord.y * TILE_SIZE.cast_signed();

            // Determine the horizontal range that falls inside the canvas.
            let first_tile_col = 0i32.max(-tile_start_x) as u32;
            let last_tile_col = TILE_SIZE.min((canvas_width.cast_signed() - tile_start_x) as u32);
            if first_tile_col >= last_tile_col {
                continue;
            }
            let buffer_start_x = (tile_start_x + first_tile_col.cast_signed()) as usize;

            for tile_row in 0..TILE_SIZE {
                let row_canvas_y = tile_start_y + tile_row.cast_signed();
                if row_canvas_y < 0 || row_canvas_y >= canvas_height.cast_signed() {
                    continue;
                }

                let src_row_start_idx = (tile_row * TILE_SIZE * 4) as usize;
                let src_copy_start_idx = src_row_start_idx + first_tile_col as usize * 4;
                let buffer_copy_start_idx = row_canvas_y as usize * stride + buffer_start_x * 4;
                let copy_len = (last_tile_col - first_tile_col) as usize * 4;

                buffer[buffer_copy_start_idx..buffer_copy_start_idx + copy_len].copy_from_slice(
                    &tile.data()[src_copy_start_idx..src_copy_start_idx + copy_len],
                );
            }
        }

        buffer
    }
}

impl Hash for Layer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
