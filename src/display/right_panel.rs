use crate::app_state::AppState;
use crate::app_state::canvas::layer::Layer;
use crate::gpu::CanvasGpu;
use crate::gpu::layers::LayerResources;
use egui::{Layout, Panel, ScrollArea, Sense, Ui, UiBuilder};
use egui_dnd::{DragAxis, dnd};

pub struct RightPanel {
    is_editing_selected_layer_name: bool,
}

impl RightPanel {
    pub fn new() -> Self {
        Self {
            is_editing_selected_layer_name: false,
        }
    }

    /// Draws the right-hand side panel containing layer controls.
    pub fn draw(&mut self, ui: &mut Ui, app_state: &mut AppState, canvas_gpu: &mut CanvasGpu) {
        Panel::right("right_panel")
            .exact_size(275.0)
            .resizable(false)
            .show(ui, |ui| {
                self.draw_layer_controls(ui, app_state, canvas_gpu);
            });
    }

    fn draw_layer_controls(
        &mut self,
        ui: &mut Ui,
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
    ) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("Layers");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([60.0, 25.0], egui::Button::new("New Layer +"))
                    .clicked()
                {
                    app_state.canvas.add_layer(canvas_gpu);
                }
            });
        });
        ui.separator();

        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let mut layers_to_delete = vec![];
            let gpu_layer_map = canvas_gpu.layers().layer_map();
            let prev_selected_layer_id = app_state
                .canvas
                .get_selected_layer()
                .map(|layer| *layer.id());
            let mut new_selected_layer_id = prev_selected_layer_id;

            let response = dnd(ui, ui.make_persistent_id("layer_controls_dnd"))
                .with_drag_axis(DragAxis::Vertical)
                .with_return_animation_time(0.0)
                .show(
                    app_state.canvas.layers_iter_mut(),
                    |ui, layer, handle, _| {
                        ui.vertical(|ui| {
                            handle.ui(ui, |ui| {
                                let layer_id = *layer.id();
                                let layer_resources = gpu_layer_map.get(&layer_id);
                                let layer_selected = prev_selected_layer_id
                                    .is_some_and(|selected_id| selected_id == layer_id);

                                self.draw_single_layer_controls(
                                    ui,
                                    layer,
                                    layer_selected,
                                    layer_resources,
                                    || new_selected_layer_id = Some(layer_id),
                                    || layers_to_delete.push(layer_id),
                                );
                            });
                        });
                    },
                );

            if response.is_drag_finished() {
                app_state.canvas.update_layers_from_dnd_response(&response);
            }

            if let Some(selected_layer_id) = new_selected_layer_id {
                app_state.canvas.set_selected_layer(selected_layer_id);
            }
            for layer_id in layers_to_delete {
                app_state.canvas.delete_layer(layer_id, canvas_gpu);
            }
        });
    }

    fn draw_single_layer_controls(
        &mut self,
        ui: &mut Ui,
        layer: &mut Layer,
        layer_selected: bool,
        layer_resources: Option<&LayerResources>,
        mut select_layer: impl FnMut(),
        delete_layer: impl FnOnce(),
    ) {
        let mut frame = egui::Frame::group(ui.style()).corner_radius(4.0);
        if layer_selected {
            frame = frame.fill(ui.visuals().selection.bg_fill);
        }

        let response = ui
            .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(layer_resources) = layer_resources {
                            ui.add(egui::Image::new(egui::load::SizedTexture::new(
                                layer_resources.texture_id,
                                egui::vec2(24.0, 24.0),
                            )));
                        } else {
                            ui.allocate_exact_size(egui::Vec2::splat(24.0), Sense::hover());
                            eprintln!(
                                "Failed to get resources for layer {} thumbnail.",
                                layer.id()
                            );
                        }

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("d").clicked() {
                                delete_layer();
                            }

                            let eye_icon = if layer.enabled { "h" } else { "s" };
                            if ui.button(eye_icon).clicked() {
                                layer.enabled = !layer.enabled;
                            }

                            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                                if self.is_editing_selected_layer_name && layer_selected {
                                    let response =
                                        ui.add(egui::TextEdit::singleline(&mut layer.name));
                                    if response.lost_focus() {
                                        self.is_editing_selected_layer_name = false;
                                    } else {
                                        response.request_focus();
                                    }
                                } else {
                                    let label_response = ui.add(
                                        egui::Label::new(&layer.name)
                                            .truncate()
                                            .sense(Sense::click()),
                                    );

                                    if label_response.double_clicked() {
                                        self.is_editing_selected_layer_name = true;
                                    }
                                }
                            });
                        });
                    });
                });
            })
            .response;

        if response.clicked() {
            select_layer();
        }
    }
}
