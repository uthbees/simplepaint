use crate::canvas::PixelColor;
use crate::canvas::canvas_pos::CanvasPos;

/// Stores and manages the state of the canvas. Data is stored in a CPU-side 2D pixel buffer.
pub struct CanvasBuffer {
    width: u32,
    height: u32,
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pixels: Vec<PixelColor>,
    /// Whether the buffer has been modified since the last GPU upload.
    pub dirty: bool,
}

impl CanvasBuffer {
    /// Creates a new canvas filled with a solid white background.
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![PixelColor::WHITE; pixel_count],
            dirty: false,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Exports the entire buffer as a flat RGBA byte slice.
    ///
    /// The slice has length `width * height * 4`.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.pixels)
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    pub fn draw_line(&mut self, pos1: CanvasPos, pos2: CanvasPos, radius: u32, color: PixelColor) {
        let radius = radius.cast_signed();
        let dx = (pos1.x - pos2.x).abs();
        let dy = (pos1.y - pos2.y).abs();
        let sx = if pos1.x < pos2.x { 1 } else { -1 };
        let sy = if pos1.y < pos2.y { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = pos1.x;
        let mut y = pos1.y;

        loop {
            if dx >= dy {
                // Shallow line: plot 1 vertical pixel strip of height 'width'
                let y_start = y - radius;
                let y_end = y + radius - 1;
                for y in y_start..=y_end {
                    self.set_pixel(x, y, color);
                }
            } else {
                // Steep line: plot 1 horizontal span of length 'width'
                let x_start = x - radius;
                let x_end = x + radius - 1;
                self.fill_scanline(y, x_start, x_end, color);
            }

            if x == pos2.x && y == pos2.y {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }

        // let radius_i32 = radius.cast_signed();
        //
        // if pos1 == pos2 {
        //     self.fill_circle(pos1, radius, color);
        //     return;
        // }
        //
        // let dx = pos2.x - pos1.x;
        // let dy = pos2.y - pos1.y;
        // let line_length = (dx * dx + dy * dy).isqrt();
        //
        // let normal_x = -dy / line_length;
        // let normal_y = dx / line_length;
        //
        // let vertices = [
        //     PxCoords {
        //         x: pos1.x + normal_x * radius_i32,
        //         y: pos1.y + normal_y * radius_i32,
        //     },
        //     PxCoords {
        //         x: pos1.x - normal_x * radius_i32,
        //         y: pos1.y - normal_y * radius_i32,
        //     },
        //     PxCoords {
        //         x: pos2.x - normal_x * radius_i32,
        //         y: pos2.y - normal_y * radius_i32,
        //     },
        //     PxCoords {
        //         x: pos2.x + normal_x * radius_i32,
        //         y: pos2.y + normal_y * radius_i32,
        //     },
        // ];
        //
        // self.fill_convex_polygon(&vertices, color);
        //
        // self.fill_circle(pos1, radius, color);
        // self.fill_circle(pos2, radius, color);
    }

    /// Fills a circle with a solid color using a scanline approach.
    ///
    /// Note: The radius is the distance from the center to the edge, so a radius 0 circle is
    /// a single pixel, and a radius 1 circle has a diameter of 3.
    pub fn fill_circle(&mut self, center: CanvasPos, radius: u32, color: PixelColor) {
        let radius = radius.cast_signed();
        for delta_y in -radius..radius {
            let y = center.y + delta_y;
            if y >= 0 {
                let delta_x = i32::isqrt(radius * radius - delta_y * delta_y);
                let start_x = center.x - delta_x;
                let end_x = center.x + delta_x;
                self.fill_scanline(y, start_x, end_x, color);
            }
        }
    }
    //
    // /// Fills a convex polygon with a solid color using a scanline approach.
    // ///
    // /// Note: Panics if `vertices` has less than two vertexes.
    // pub fn fill_convex_polygon(&mut self, vertices: &[PxCoords], color: PixelColor) {
    //     let vertex_count = u32::try_from(vertices.len())
    //         .expect("Should be passed a reasonable number of vertices");
    //     assert!(vertex_count >= 2);
    //
    //     let mut min_y = i32::MAX;
    //     let mut max_y = 0;
    //
    //     for vertex in vertices {
    //         if vertex.y < min_y {
    //             min_y = vertex.y;
    //         }
    //         if vertex.y > max_y {
    //             max_y = vertex.y;
    //         }
    //     }
    //
    //     for y in min_y..=max_y {
    //         let mut min_x = i32::MAX;
    //         let mut max_x = 0;
    //
    //         // Intersect the current horizontal scanline with each polygon edge to find the boundaries.
    //         for pair in vertices.windows(2) {
    //             if let Some(x) = Self::find_scanline_intersection(y, pair[0], pair[1]) {
    //                 min_x = i32::min(min_x, x);
    //                 max_x = i32::max(max_x, x);
    //             }
    //         }
    //         let first = vertices.first().expect("Asserted that vertices.len() >= 2");
    //         let last = vertices.last().expect("Asserted that vertices.len() >= 2");
    //         if let Some(x) = Self::find_scanline_intersection(y, *last, *first) {
    //             min_x = i32::min(min_x, x);
    //             max_x = i32::max(max_x, x);
    //         }
    //
    //         self.fill_scanline(y, min_x, max_x, color);
    //     }
    // }

    #[allow(clippy::cast_sign_loss)]
    /// Sets the pixel at `(x, y)` to the given color.
    ///
    /// Coordinates outside the canvas are ignored.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: PixelColor) {
        if x >= 0 && x < self.width.cast_signed() && y >= 0 && y < self.height.cast_signed() {
            self.dirty = true;
            self.pixels[(y * self.width.cast_signed() + x) as usize] = color;
        }
    }

    /// Fills a scanline from `start_x` to `end_x` with a solid color.
    ///
    /// Writes outside the canvas are ignored.
    fn fill_scanline(&mut self, scan_y: i32, start_x: i32, end_x: i32, color: PixelColor) {
        if start_x <= end_x
            && end_x >= 0
            && start_x < self.width.cast_signed()
            && scan_y >= 0
            && scan_y < self.height.cast_signed()
        {
            self.dirty = true;

            let start_x = i32::max(start_x, 0).cast_unsigned();
            let end_x = i32::min(end_x, self.width.cast_signed()).cast_unsigned();

            let row_start_idx = scan_y.cast_unsigned() * self.width;
            let start_idx = (row_start_idx + start_x) as usize;
            let end_idx = (row_start_idx + end_x) as usize;
            self.pixels[start_idx..=end_idx].fill(color);
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    /// Given the y coordinate of a horizontal scanline and a line defined by two vertices, finds the
    /// intersection of the two lines.
    /// Returns None if the lines don't intersect.
    fn find_scanline_intersection(scan_y: i32, vert1: CanvasPos, vert2: CanvasPos) -> Option<i32> {
        // If the two lines intersect...
        if (vert1.y <= scan_y && vert2.y > scan_y) || (vert2.y <= scan_y && vert1.y > scan_y) {
            // ...find the percentage of the way the intersection is along the vertex line...
            let intersect_percent = (scan_y - vert1.y) as f32 / (vert2.y - vert1.y) as f32;
            // ...and convert that percentage into an x coordinate.
            let x = vert1.x + f32::round(intersect_percent * (vert2.x - vert1.x) as f32) as i32;
            Some(x)
        } else {
            None
        }
    }
}