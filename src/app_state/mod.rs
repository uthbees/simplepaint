use crate::app_state::canvas::Canvas;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui::{Rgba, Vec2};
use std::collections::VecDeque;

pub mod canvas;

const DEFAULT_CANVAS_HEIGHT: u32 = 500;
const DEFAULT_CANVAS_WIDTH: u32 = 500;

/// Manages the user-determined parts of state. Does not manage rendering, layout, or other
/// technical aspects of the app, and does not contain any handles to managers of those aspects.
pub struct AppState {
    pub foreground_color: Rgba,
    pub background_color: Rgba,
    recent_colors: VecDeque<Option<Rgba>>,
    pub palette_colors: [Rgba; 10],

    pub canvas: Canvas,
    pub canvas_view: CanvasView,
}

pub struct CanvasView {
    pub dimensions: DimensionsPxNonzero,
    /// The x/y pan offset of the center of the canvas from the center of the panel.
    /// Measured in canvas pixels. (0, 0) puts the center of the canvas in the center of the panel.
    pan_px: Vec2,
    /// Canvas zoom. Larger numbers zoom in, smaller numbers zoom out.
    /// 1 is 1:1 screen px:canvas px, 2 is 2:1 screen px:canvas px, 0.5 is 1:2 screen px:canvas px.
    zoom: f32,
}

impl AppState {
    pub fn new() -> AppState {
        let mut recent_colors = VecDeque::with_capacity(10);
        recent_colors.resize(10, None);

        AppState {
            foreground_color: Rgba::BLACK,
            background_color: Rgba::WHITE,
            canvas: Canvas::new(DimensionsPxNonzero::new(
                DEFAULT_CANVAS_WIDTH,
                DEFAULT_CANVAS_HEIGHT,
            )),
            recent_colors,
            palette_colors: [
                Rgba::BLACK,
                Rgba::WHITE,
                Rgba::from_rgb(1.0, 0.0, 0.0),
                Rgba::from_rgb(0.0, 1.0, 0.0),
                Rgba::from_rgb(0.0, 0.0, 1.0),
                Rgba::from_rgb(1.0, 1.0, 0.0),
                Rgba::from_rgb(1.0, 0.0, 1.0),
                Rgba::from_rgb(0.0, 1.0, 1.0),
                Rgba::from_rgb(1.0, 0.5, 0.0),
                Rgba::from_rgb(0.5, 0.0, 0.5),
            ],
            canvas_view: CanvasView::new(),
        }
    }

    /// Adds a color to the front of the recent colors list, deduplicating and
    /// capping at 10.
    pub fn push_recent_color(&mut self, new_color: Rgba) {
        // Remove any existing recent color that is within epsilon of the new color.
        let epsilon = 0.00001;
        self.recent_colors.retain(|recent| {
            match recent {
                Some(existing) => {
                    let a = existing.to_array();
                    let b = new_color.to_array();

                    // Check if all RGBA channels are within epsilon of each other
                    let is_close = a
                        .iter()
                        .zip(b.iter())
                        .all(|(x, y)| (x - y).abs() <= epsilon);

                    // Keep items that are NOT close (i.e., remove duplicates)
                    !is_close
                }
                None => true,
            }
        });

        self.recent_colors.insert(0, Some(new_color));
        self.recent_colors.resize(10, None);
    }

    pub fn get_recent_colors(&self) -> Vec<Option<Rgba>> {
        self.recent_colors.iter().copied().collect::<Vec<_>>()
    }
}

impl CanvasView {
    fn new() -> Self {
        Self {
            dimensions: DimensionsPxNonzero::new(1, 1),
            pan_px: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    pub fn pan_px(&self) -> Vec2 {
        self.pan_px
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn set_pan_px(&mut self, pan_px: Vec2, canvas: &Canvas) {
        self.pan_px = pan_px;
        self.clamp_pan(canvas);
    }

    pub fn set_pan_px_x(&mut self, pan_px_x: f32, canvas: &Canvas) {
        self.pan_px.x = pan_px_x;
        self.clamp_pan(canvas);
    }

    pub fn set_pan_px_y(&mut self, pan_px_y: f32, canvas: &Canvas) {
        self.pan_px.y = pan_px_y;
        self.clamp_pan(canvas);
    }

    #[allow(dead_code)]
    pub fn set_zoom(&mut self, zoom: f32, canvas: &Canvas) {
        self.zoom = zoom;
        self.clamp_pan(canvas);
    }

    /// Allows setting both pan and zoom at the same time, only clamping after setting both.
    pub fn set_pan_and_zoom(&mut self, pan_px: Vec2, zoom: f32, canvas: &Canvas) {
        self.pan_px = pan_px;
        self.zoom = zoom;
        self.clamp_pan(canvas);
    }

    /// Clamps the pan to prevent the edges of the canvas from going beyond the center of the panel.
    fn clamp_pan(&mut self, canvas: &Canvas) {
        let canvas_dim = canvas.dimensions();

        let half_w = canvas_dim.width() as f32 * 0.5;
        let half_h = canvas_dim.height() as f32 * 0.5;

        self.pan_px.x = self.pan_px.x.clamp(-half_w, half_w);
        self.pan_px.y = self.pan_px.y.clamp(-half_h, half_h);
    }

    /// Calculates the current normalization factors for the canvas view. These factors can be
    /// multiplied with the starting uvs (numbers ranging linearly from 0 to 1 across the
    /// canvas panel coordinates) to normalize the points to a canvas with the proper aspect ratio
    /// and a 1:1 canvas pixel:screen pixel scale.
    /// Note that these factors do not account for zooming or panning.
    pub fn get_normalization_factors(&self, canvas: &Canvas) -> [f32; 2] {
        let canvas_dim = canvas.dimensions();

        [
            self.dimensions.width() as f32 / canvas_dim.width() as f32,
            self.dimensions.height() as f32 / canvas_dim.height() as f32,
        ]
    }
}
