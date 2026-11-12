use crate::app_state::{MAX_CANVAS_HEIGHT, MAX_CANVAS_WIDTH};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Add;
use tiny_skia::{Color, Pixmap};

/// Side length of every square tile, in pixels.
pub const TILE_SIZE: u32 = 256;

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
        let dimensions = dimensions.clamp_max_values(MAX_CANVAS_WIDTH, MAX_CANVAS_HEIGHT);

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

    /// Returns a mutable reference to the tile at `coord` if `coord` is at least partially in the
    /// canvas, creating a new tile filled with the layer's `default_color` if one does not already
    /// exist. Marks the tile and layer as dirty unless returning None.
    pub fn get_tile_mut(&mut self, coord: TileCoord) -> Option<&mut Pixmap> {
        let tile_size = TILE_SIZE.cast_signed();

        let origin_x = coord.x * tile_size;
        let origin_y = coord.y * tile_size;
        if origin_x >= self.dimensions.width().cast_signed()
            || origin_y >= self.dimensions.height().cast_signed()
            || origin_x + tile_size - 1 < 0
            || origin_y + tile_size - 1 < 0
        {
            return None;
        }

        self.dirty = true;
        self.dirty_tiles.insert(coord);
        Some(self.tiles.entry(coord).or_insert_with(|| {
            let mut pm = Pixmap::new(TILE_SIZE, TILE_SIZE)
                .expect("Tile pixmap creation should succeed for TILE_SIZE > 0");
            pm.fill(self.default_color);
            pm
        }))
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
            let default_color_pm = self.default_color.premultiply().to_color_u8();
            for pixel in buffer.as_chunks_mut::<4>().0 {
                pixel[0] = default_color_pm.red();
                pixel[1] = default_color_pm.green();
                pixel[2] = default_color_pm.blue();
                pixel[3] = default_color_pm.alpha();
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

    pub fn resize(&mut self, new_dimensions: DimensionsPxNonzero, canvas_gpu: &mut CanvasGpu) {
        let new_dimensions = new_dimensions.clamp_max_values(MAX_CANVAS_WIDTH, MAX_CANVAS_HEIGHT);

        // Drop any tiles that will no longer fall within the canvas, and clear the non-visible
        // contents of any that extend past an edge.
        if new_dimensions.width() < self.dimensions.width() {
            self.drop_tiles_in_rect(
                Self::canvas_to_tile_coord(new_dimensions.width().cast_signed() - 1, 0)
                    + TileCoord { x: 1, y: 0 },
                Self::canvas_to_tile_coord(
                    self.dimensions.width().cast_signed() - 1,
                    self.dimensions.height().cast_signed() - 1,
                ),
            );
            if !new_dimensions.width().is_multiple_of(TILE_SIZE) {
                let tile_x = new_dimensions.width() / TILE_SIZE;
                let clear_start_x_in_tile = new_dimensions.width() % TILE_SIZE;
                for tile_y in 0..=(new_dimensions.height() / TILE_SIZE) {
                    self.clear_tile_from_coord(
                        TileCoord {
                            x: tile_x.cast_signed(),
                            y: tile_y.cast_signed(),
                        },
                        clear_start_x_in_tile,
                        &ClearTileDirection::Right,
                    );
                }
            }
        }
        if new_dimensions.height() < self.dimensions.height() {
            self.drop_tiles_in_rect(
                Self::canvas_to_tile_coord(0, new_dimensions.height().cast_signed() - 1)
                    + TileCoord { x: 0, y: 1 },
                Self::canvas_to_tile_coord(
                    self.dimensions.width().cast_signed() - 1,
                    self.dimensions.height().cast_signed() - 1,
                ),
            );
            if !new_dimensions.height().is_multiple_of(TILE_SIZE) {
                let tile_y = new_dimensions.height() / TILE_SIZE;
                let clear_start_y_in_tile = new_dimensions.height() % TILE_SIZE;
                for tile_x in 0..=(new_dimensions.width() / TILE_SIZE) {
                    self.clear_tile_from_coord(
                        TileCoord {
                            x: tile_x.cast_signed(),
                            y: tile_y.cast_signed(),
                        },
                        clear_start_y_in_tile,
                        &ClearTileDirection::Down,
                    );
                }
            }
        }

        // Update the dimensions after doing our calculations that need the old dimensions, but
        // before calling any functions that need the new dimensions.
        self.dimensions = new_dimensions;

        // Mark this layer as not dirty, as calling resize_layer puts the layer texture into a
        // fully up-to-date state.
        self.clear_dirty();
        canvas_gpu
            .layers()
            .resize_layer(self.id, new_dimensions, &self.buffer_as_bytes());
    }

    fn drop_tiles_in_rect(&mut self, rect_min: TileCoord, rect_max: TileCoord) {
        for x in rect_min.x..=rect_max.x {
            for y in rect_min.y..=rect_max.y {
                self.tiles.remove(&TileCoord { x, y });
            }
        }
    }

    /// In one axis, clears a tile starting at the specified coordinate within the tile and
    /// continuing until the edge of the tile in the specified direction.
    fn clear_tile_from_coord(
        &mut self,
        tile_coord: TileCoord,
        clear_start_coord: u32,
        direction: &ClearTileDirection,
    ) {
        let tile_size = TILE_SIZE as usize;
        let clear_start_coord = clear_start_coord as usize;
        let default_color = self.default_color.premultiply().to_color_u8();

        if let Some(tile) = self.tiles.get_mut(&tile_coord) {
            let data = tile.pixels_mut();

            match direction {
                ClearTileDirection::Right => {
                    for y in 0..tile_size {
                        let row_start = y * tile_size;
                        data[row_start + clear_start_coord..row_start + tile_size]
                            .fill(default_color);
                    }
                }
                ClearTileDirection::Down => {
                    for y in clear_start_coord..tile_size {
                        let row_start = y * tile_size;
                        data[row_start..row_start + tile_size].fill(default_color);
                    }
                }
            }
        }
    }

    /// Converts canvas pixel coordinates to tile coordinates.
    ///
    /// Uses Euclidean (floor) division so negative canvas coordinates map correctly.
    pub fn canvas_to_tile_coord(canvas_x: i32, canvas_y: i32) -> TileCoord {
        let tile_size = TILE_SIZE.cast_signed();

        TileCoord {
            x: canvas_x.div_euclid(tile_size),
            y: canvas_y.div_euclid(tile_size),
        }
    }
}

impl Hash for Layer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Integer coordinate identifying a tile in the sparse grid.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

impl Add for TileCoord {
    type Output = TileCoord;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

enum ClearTileDirection {
    Right,
    Down,
}
