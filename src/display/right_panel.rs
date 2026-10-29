use crate::app_state::AppState;
use egui::{Panel, Ui};

pub struct RightPanel {}

impl RightPanel {
    /// Draws the right-hand side panel containing layer controls.
    pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
        Panel::right("right_panel")
            .exact_size(250.0)
            .resizable(false)
            .show(ui, |ui| {
                Self::draw_layer_controls(ui, app_state);
            });
    }

    fn draw_layer_controls(ui: &mut Ui, app_state: &mut AppState) {}
}
