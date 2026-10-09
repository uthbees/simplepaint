// Sizes here are the sizes on the short side, i.e. the side that doesn't resize with the window.
pub const SCROLLBAR_TRACK_SIZE_PT: f32 = 10.0;
const THUMB_SIZE_PT: f32 = 6.0;

const THUMB_MIN_LENGTH_PT: f32 = 20.0;

const THUMB_INSET_PT: f32 = (SCROLLBAR_TRACK_SIZE_PT - THUMB_SIZE_PT) * 0.5;

pub const SCROLLBAR_TRACK_COLOR: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 100);
const THUMB_COLOR: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(180, 180, 180, 160);
const THUMB_HOVER_COLOR: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(200, 200, 200, 200);
const THUMB_ACTIVE_COLOR: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(220, 220, 220, 230);

#[derive(Clone, Copy)]
pub enum ScrollbarAxis {
    Vertical,
    Horizontal,
}

pub struct Scrollbar {
    axis: ScrollbarAxis,
    /// If dragging the scrollbar, the position of the cursor within the thumb, otherwise None.
    /// Stores only the position in the long dimension, i.e. the dimension that resizes with the window.
    thumb_grab_offset: Option<f32>,
}

impl Scrollbar {
    pub fn new(axis: ScrollbarAxis) -> Self {
        Self {
            axis,
            thumb_grab_offset: None,
        }
    }

    /// Draws a single scrollbar (vertical or horizontal) in the given track rect.
    ///
    /// * `track_rect`: The rectangle to render the scrollbar track into. Should respect
    ///   `SCROLLBAR_TRACK_SIZE_PT`.
    /// * `viewport_fraction`: The space shown by the viewport divided by the total scrollable space
    ///   for this scrollbar's axis. Should have a range of (0, 1].
    /// * `pan`: The pan of the viewport on the scrollbar's axis, normalized to [0, 1].
    ///
    /// Returns Some with the new pan value if the scrollbar was moved, otherwise None.
    #[must_use]
    pub fn draw(
        &mut self,
        egui_ui: &mut egui::Ui,
        track_rect: egui::Rect,
        viewport_fraction: f32,
        pan: f32,
    ) -> Option<f32> {
        let track_length = match self.axis {
            ScrollbarAxis::Vertical => track_rect.height(),
            ScrollbarAxis::Horizontal => track_rect.width(),
        };
        let thumb_length =
            (track_length * viewport_fraction).clamp(THUMB_MIN_LENGTH_PT, track_length);
        let max_thumb_offset = track_length - thumb_length;

        let response = egui_ui.allocate_rect(track_rect, egui::Sense::DRAG);

        if response.drag_started()
            && let Some(pointer_pos_2d) = egui_ui.input(|i| i.pointer.interact_pos())
        {
            let pointer_pos = match self.axis {
                ScrollbarAxis::Vertical => pointer_pos_2d.y,
                ScrollbarAxis::Horizontal => pointer_pos_2d.x,
            };
            let thumb_offset = pan * max_thumb_offset;
            if pointer_pos >= thumb_offset && pointer_pos <= thumb_offset + thumb_length {
                // Grabbed the thumb - save the offset within the thumb that we're dragging from.
                self.thumb_grab_offset = Some(pointer_pos - thumb_offset);
            } else {
                // Clicked on the track - jump the center of the thumb to the click position and
                // start dragging from there.
                self.thumb_grab_offset = Some(thumb_length * 0.5);
            }
        }

        let mut updated_pan = None;
        if response.dragged()
            && let (Some(grab_offset), Some(pointer_pos_2d)) = (
                self.thumb_grab_offset,
                egui_ui.input(|i| i.pointer.interact_pos()),
            )
        {
            let thumb_offset = match self.axis {
                ScrollbarAxis::Vertical => pointer_pos_2d.y - track_rect.top(),
                ScrollbarAxis::Horizontal => pointer_pos_2d.x - track_rect.left(),
            } - grab_offset;

            let new_normalized_pan = if max_thumb_offset > 0.0 {
                (thumb_offset / max_thumb_offset).clamp(0.0, 1.0)
            } else {
                0.5
            };

            updated_pan = Some(new_normalized_pan);
        }

        if response.drag_stopped() {
            self.thumb_grab_offset = None;
        }

        let thumb_offset = updated_pan.unwrap_or(pan) * max_thumb_offset;
        let thumb_rect = self.build_thumb_rect(track_rect, thumb_offset, thumb_length);

        let is_hovered = response.hovered()
            && egui_ui
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|pos| thumb_rect.contains(pos));
        let thumb_color = if self.thumb_grab_offset.is_some() {
            THUMB_ACTIVE_COLOR
        } else if is_hovered {
            THUMB_HOVER_COLOR
        } else {
            THUMB_COLOR
        };

        egui_ui
            .painter()
            .rect_filled(track_rect, 0.0, SCROLLBAR_TRACK_COLOR);
        egui_ui.painter().rect_filled(thumb_rect, 3.0, thumb_color);

        updated_pan
    }

    fn build_thumb_rect(
        &self,
        track_rect: egui::Rect,
        progress_offset: f32,
        thumb_length: f32,
    ) -> egui::Rect {
        match self.axis {
            ScrollbarAxis::Vertical => egui::Rect::from_min_size(
                egui::pos2(
                    track_rect.left() + THUMB_INSET_PT,
                    track_rect.top() + progress_offset,
                ),
                egui::vec2(THUMB_SIZE_PT, thumb_length),
            ),
            ScrollbarAxis::Horizontal => egui::Rect::from_min_size(
                egui::pos2(
                    track_rect.left() + progress_offset,
                    track_rect.top() + THUMB_INSET_PT,
                ),
                egui::vec2(thumb_length, THUMB_SIZE_PT),
            ),
        }
    }
}
