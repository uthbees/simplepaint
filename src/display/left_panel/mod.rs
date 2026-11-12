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

    fn draw_tools(ui: &mut Ui, app_state: &mut AppState) {
        ui.add_space(8.0);
        ui.heading("Tools");
        ui.separator();

        ui.add_space(8.0);
        ui.label(RichText::new("Tool").strong());
        ui.label("Paintbrush");

        ui.separator();

        ui.label(RichText::new("Paintbrush settings").strong());
        ui.checkbox(&mut app_state.tool_settings.anti_alias, "Anti-alias");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Thickness:"));
            ui.add(egui::DragValue::new(&mut app_state.tool_settings.stroke_width).range(1..=1000));
        });
    }
}
