use bytemuck::{Pod, Zeroable};
use egui::Pos2;
use std::cmp::PartialEq;
use std::ops::Add;

/// A single RGBA pixel stored as four `u8` bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PixelColor {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
}

#[derive(Copy, Clone, Eq, Debug)]
/// A position in pixels relative to the canvas.
///
/// Note that positions off the canvas are valid values for `PxCoords`. This includes
/// negative coordinates, even though all positions displayed on the canvas are positive.
struct PxCoords {
    x: i32,
    y: i32,
}

/// Stores and manages the state of the canvas. Data is stored in a CPU-side 2D pixel buffer.
pub struct Canvas {
    width: u32,
    height: u32,
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pixels: Vec<PixelColor>,
}

impl Add for PxCoords {
    type Output = PxCoords;

    fn add(self, rhs: Self) -> Self::Output {
        PxCoords {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl PartialEq for PxCoords {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Canvas {
    /// Creates a new canvas filled with a solid white background.
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![PixelColor::WHITE; pixel_count],
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

    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    /// Converts a position from logical points to pixel coordinates.
    ///
    /// * `pos` - The position to convert, in logical points.
    /// * `canvas_rect` - The egui canvas rectangle.
    fn points_to_px(&self, pos_x: f32, pos_y: f32, canvas_rect: egui::Rect) -> PxCoords {
        let x_percent = pos_x / canvas_rect.width();
        let y_percent = pos_y / canvas_rect.height();
        let x_px = (x_percent * self.width() as f32) as i32;
        let y_px = (y_percent * self.height() as f32) as i32;
        PxCoords { x: x_px, y: y_px }
    }

    #[must_use]
    /// Handles input to the canvas. Returns true if the canvas is modified, otherwise false.
    pub fn handle_input(
        &mut self,
        ui: &mut egui::Ui,
        canvas_response: Option<egui::Response>,
    ) -> bool {
        if let Some(response) = canvas_response
            && response.dragged()
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            let delta = response.drag_delta();
            let rect = response.rect;

            let relative_pos = (pos.x - rect.min.x, pos.y - rect.min.y);
            let pos_px = self.points_to_px(relative_pos.0, relative_pos.1, rect);
            let delta_px = self.points_to_px(delta.x, delta.y, rect);

            self.draw_line(pos_px, pos_px + delta_px, 1, PixelColor::BLACK);
            // testing lines
            // self.draw_line(
            //     PxCoords { x: 1, y: 1 },
            //     PxCoords { x: 1, y: 1 },
            //     1,
            //     PixelColor::BLACK,
            // );
            // self.draw_line(
            //     PxCoords { x: 10, y: 10 },
            //     PxCoords { x: 50, y: 50 },
            //     1,
            //     PixelColor::BLACK,
            // );
            // self.draw_line(
            //     PxCoords { x: 10, y: 10 },
            //     PxCoords { x: 25, y: 50 },
            //     1,
            //     PixelColor::BLACK,
            // );
            // self.draw_line(
            //     PxCoords { x: 10, y: 10 },
            //     PxCoords { x: 50, y: 25 },
            //     1,
            //     PixelColor::BLACK,
            // );
            return true;
        }

        false
    }

    #[allow(clippy::cast_sign_loss)]
    /// Sets the pixel at `(x, y)` to the given color.
    ///
    /// Coordinates outside the canvas are ignored.
    fn set_pixel(&mut self, x: i32, y: i32, color: PixelColor) {
        if x >= 0 && x < self.width.cast_signed() && y >= 0 && y < self.height.cast_signed() {
            self.pixels[(y * self.width.cast_signed() + x) as usize] = color;
        }
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    fn draw_line(&mut self, pos1: PxCoords, pos2: PxCoords, radius: u32, color: PixelColor) {
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
    fn fill_circle(&mut self, center: PxCoords, radius: u32, color: PixelColor) {
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
    // fn fill_convex_polygon(&mut self, vertices: &[PxCoords], color: PixelColor) {
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

    /// Fills a scanline from `start_x` to `end_x` with a solid color.
    ///
    /// Writes outside the canvas are ignored.
    fn fill_scanline(&mut self, scan_y: i32, start_x: i32, end_x: i32, color: PixelColor) {
        if start_x <= end_x
            && end_x >= 0
            && start_x <= self.width.cast_signed()
            && scan_y >= 0
            && scan_y <= self.height.cast_signed()
        {
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
    fn find_scanline_intersection(scan_y: i32, vert1: PxCoords, vert2: PxCoords) -> Option<i32> {
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
