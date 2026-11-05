use crate::app_state::AppState;
use crate::app_state::canvas::layer::Layer;
use crate::gpu::CanvasGpu;
use crate::gpu::layers::LayerResources;
use egui::{Layout, Panel, ScrollArea, Sense, Ui, UiBuilder, Vec2};
use egui_dnd::{DragAxis, dnd};
use egui_material_icons::icons;

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
                    .add_sized(
                        [60.0, 25.0],
                        egui::Button::new(("New Layer", icons::ICON_ADD)),
                    )
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
            let mut interactable_hovered = false;

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

                                let results = self.draw_single_layer_controls(
                                    ui,
                                    layer,
                                    layer_selected,
                                    layer_resources,
                                );

                                if results.selected {
                                    new_selected_layer_id = Some(layer_id);
                                }
                                if results.delete_clicked {
                                    layers_to_delete.push(layer_id);
                                }
                                if results.interactable_hovered {
                                    interactable_hovered = true;
                                }
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
            // We have to set the cursor icon out here so that we override the cursor set by egui_dnd
            // instead of the other way around.
            if interactable_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }
        });
    }

    fn draw_single_layer_controls(
        &mut self,
        ui: &mut Ui,
        layer: &mut Layer,
        layer_selected: bool,
        layer_resources: Option<&LayerResources>,
    ) -> DrawLayerResults {
        let mut results = DrawLayerResults {
            selected: false,
            delete_clicked: false,
            interactable_hovered: false,
        };

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
                            ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                            eprintln!(
                                "Failed to get resources for layer {} thumbnail.",
                                layer.id()
                            );
                        }

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().button_padding = Vec2 { x: 3.0, y: 0.0 };
                            ui.spacing_mut().item_spacing = Vec2 { x: 5.0, y: 5.0 };
                            let delete_response = ui.button(icons::ICON_DELETE);
                            if delete_response.clicked() {
                                results.delete_clicked = true;
                            }

                            let eye_icon = if layer.enabled {
                                icons::ICON_VISIBILITY
                            } else {
                                icons::ICON_VISIBILITY_OFF
                            };
                            let eye_response = ui.button(eye_icon);
                            if eye_response.clicked() {
                                layer.enabled = !layer.enabled;
                            }

                            if delete_response.hovered() || eye_response.hovered() {
                                results.interactable_hovered = true;
                            }

                            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                                if self.is_editing_selected_layer_name && layer_selected {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut layer.name)
                                            .margin(egui::Margin::symmetric(4, 4)),
                                    );
                                    if response.lost_focus() {
                                        self.is_editing_selected_layer_name = false;
                                    } else {
                                        response.request_focus();
                                    }
                                    if response.hovered() {
                                        results.interactable_hovered = true;
                                    }
                                } else {
                                    let response = egui::Frame::new()
                                        .inner_margin(egui::Margin::symmetric(0, 4))
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&layer.name)
                                                        .color(egui::Color32::WHITE),
                                                )
                                                .truncate(),
                                            );
                                        })
                                        .response;
                                    let response = response.interact(Sense::click());

                                    if response.double_clicked() {
                                        self.is_editing_selected_layer_name = true;
                                    }
                                    if response.hovered() {
                                        results.interactable_hovered = true;
                                    }
                                }
                            });
                        });
                    });
                });
            })
            .response;

        if response.clicked() {
            results.selected = true;
        }

        results
    }
}

struct DrawLayerResults {
    selected: bool,
    delete_clicked: bool,
    interactable_hovered: bool,
}
