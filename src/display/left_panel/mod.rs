mod colors;

use crate::app_state::AppState;
use crate::display::left_panel::colors::ColorControls;
use egui::{Panel, RichText, Ui};

pub struct LeftPanel {}

impl LeftPanel {
    /// Draws the left-hand side panel containing basic tool/color indicators.
    pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
        Panel::left("left_panel")
            .exact_size(230.0)
            .resizable(false)
            .show(ui, |ui| {
                Panel::bottom("color_panel")
                    .exact_size(200.0)
                    .resizable(false)
                    .show(ui, |ui| {
                        ColorControls::draw(ui, app_state);
                    });
                Panel::top("tools_panel").resizable(false).show(ui, |ui| {
                    Self::draw_tools(ui, app_state);
                });
            });
    }

    #[allow(unused_variables)]
    fn draw_tools(ui: &mut Ui, app_state: &mut AppState) {
        ui.add_space(8.0);
        ui.heading("SimplePaint");
        ui.separator();

        ui.add_space(8.0);
        ui.label(RichText::new("Tool").strong());
        ui.label("Paintbrush");
    }
}
