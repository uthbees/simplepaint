use crate::ui::ui_state::UiState;
use egui::{Panel, Ui};

pub struct RightPanel {}

impl RightPanel {
    /// Draws the right-hand side panel containing layer controls.
    pub fn draw(ui: &mut Ui, ui_state: &mut UiState) {
        Panel::right("right_panel")
            .exact_size(250.0)
            .resizable(false)
            .show(ui, |ui| {
                Self::draw_layer_controls(ui, ui_state);
            });
    }

    fn draw_layer_controls(ui: &mut Ui, ui_state: &mut UiState) {}
}
