use crate::app_state::AppState;
use egui::{Panel, ScrollArea, Ui};
use egui_dnd::dnd;

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

    fn draw_layer_controls(ui: &mut Ui, app_state: &mut AppState) {
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let response = dnd(ui, ui.make_persistent_id("layer_controls_dnd")).show(
                app_state.canvas.layers().iter(),
                |ui, item, handle, drag_state| {
                    ui.vertical(|ui| {
                        handle.ui(ui, |ui| {
                            if drag_state.dragged {
                                ui.label("dragging");
                            } else {
                                ui.label("drag");
                            }
                        });
                        ui.label(&item.name);
                    });
                },
            );

            if response.is_drag_finished() {
                app_state.canvas.update_layers_from_dnd_response(&response);
            }
        });
    }
}
