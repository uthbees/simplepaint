use crate::app_state::AppState;
use crate::app_state::canvas::layer::Layer;
use crate::gpu::CanvasGpu;
use crate::gpu::layers::LayerResources;
use egui::{
    Align, Atom, Button, FontId, IdSalt, Layout, Margin, Panel, RichText, ScrollArea, Sense, Ui,
    UiBuilder, Vec2,
};
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
                Self::draw_layer_panel_header(ui, app_state, canvas_gpu);
                Self::draw_layer_buttons_panel(ui);
                self.draw_layer_widgets(ui, app_state, canvas_gpu);
            });
    }

    fn draw_layer_panel_header(ui: &mut Ui, app_state: &mut AppState, canvas_gpu: &mut CanvasGpu) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("Layers");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_sized([60.0, 25.0], Button::new(("New Layer", icons::ICON_ADD)))
                    .clicked()
                {
                    app_state.canvas.add_layer(canvas_gpu);
                }
            });
        });
        ui.separator();
    }

    fn draw_layer_buttons_panel(ui: &mut Ui) {
        Panel::bottom("layer_buttons_panel")
            .frame(
                egui::Frame::side_top_panel(ui.style()).inner_margin(Margin {
                    left: 0,
                    right: 0,
                    top: 6,
                    bottom: 4,
                }),
            )
            .show(ui, |ui| {
                ui.spacing_mut().button_padding = Vec2 { x: 8.0, y: 2.0 };
                ui.columns(6, |columns| {
                    columns[0].vertical_centered(|ui| {
                        // TODO: disable when none selected
                        let response = ui.add_enabled(
                            true,
                            Button::new(RichText::new(icons::ICON_CONTENT_COPY).size(20.0)),
                        );

                        if response.clicked() {
                            todo!()
                        }
                        response.on_hover_text("Duplicate selected layer");
                    });
                    columns[1].vertical_centered(|ui| {
                        // TODO: disable when none selected or none above
                        let response = ui.add_enabled(
                            true,
                            Button::new(RichText::new(icons::ICON_MERGE).size(20.0)),
                        );

                        if response.clicked() {
                            todo!()
                        }
                        response.on_hover_text("Merge selected layer into layer above");
                    });
                    columns[2].vertical_centered(|ui| {
                        ui.style_mut().visuals.override_text_color =
                            Some(egui::Color32::TRANSPARENT);

                        // TODO: disable when none selected or none below
                        let response = ui.add_enabled(
                            true,
                            Button::new(RichText::new(icons::ICON_MERGE).size(20.0)),
                        );

                        Self::draw_upside_down_merge_icon(
                            ui,
                            response.rect,
                            FontId::proportional(20.0),
                            ui.style().interact(&response).text_color(),
                        );

                        if response.clicked() {
                            todo!()
                        }
                        response.on_hover_text("Merge selected layer into layer below");
                    });
                    columns[3].vertical_centered(|ui| {
                        // TODO: disable when none selected or none above
                        let response = ui.add_enabled(
                            true,
                            Button::new(RichText::new(icons::ICON_NORTH).size(20.0)),
                        );

                        if response.clicked() {
                            todo!()
                        }
                        response.on_hover_text("Move selected layer up");
                    });
                    columns[4].vertical_centered(|ui| {
                        // TODO: disable when none selected or none below
                        let response = ui.add_enabled(
                            true,
                            Button::new(RichText::new(icons::ICON_SOUTH).size(20.0)),
                        );

                        if response.clicked() {
                            todo!()
                        }
                        response.on_hover_text("Move selected layer down");
                    });
                    columns[5].vertical_centered(|ui| {
                        // TODO: disable when none selected
                        let response = ui.add_enabled(
                            true,
                            Button::new(RichText::new(icons::ICON_DRIVE_FILE_RENAME).size(20.0)),
                        );

                        if response.clicked() {
                            todo!()
                        }
                        response.on_hover_text("Rename selected layer");
                    });
                });
            });
    }

    fn draw_layer_widgets(
        &mut self,
        ui: &mut Ui,
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
    ) {
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

                                let results = self.draw_single_layer_widget(
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

    fn draw_single_layer_widget(
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

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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

                            eye_response.on_hover_text("Show/hide layer");
                            delete_response.on_hover_text("Delete layer");

                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                if self.is_editing_selected_layer_name && layer_selected {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut layer.name)
                                            .margin(Margin::symmetric(4, 4)),
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
                                        .inner_margin(Margin::symmetric(0, 4))
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&layer.name)
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

        response.container_context_menu(|ui| {
            self.draw_layer_widget_context_menu(ui);
        });

        if response.clicked() {
            results.selected = true;
        }

        results
    }

    fn draw_layer_widget_context_menu(&self, ui: &mut Ui) {
        if ui.button((icons::ICON_CONTENT_COPY, "Duplicate")).clicked() {
            todo!();
            ui.close();
        }

        // TODO: disable if nothing above
        if ui
            .add_enabled(true, Button::new((icons::ICON_MERGE, "Merge up")))
            .clicked()
        {
            todo!();
            ui.close();
        }

        let font_id = FontId::proportional(egui::TextStyle::Button.resolve(ui.style()).size);
        let merge_down_icon_galley = ui.painter().layout_no_wrap(
            icons::ICON_MERGE.codepoint.into(),
            font_id.clone(),
            egui::Color32::TRANSPARENT,
        );
        let merge_down_icon_id = IdSalt::new("merge_down_icon_id");
        let merge_down_atom = Atom::custom(merge_down_icon_id, merge_down_icon_galley.size());

        // TODO: disable when nothing below
        let response = ui
            .add_enabled_ui(true, |ui| {
                Button::new((merge_down_atom, "Merge down")).atom_ui(ui)
            })
            .inner;
        if let Some(rect) = response.rect(merge_down_icon_id) {
            Self::draw_upside_down_merge_icon(
                ui,
                rect,
                font_id,
                ui.style().interact(&response).text_color(),
            );
        }

        if response.clicked() {
            todo!();
            ui.close();
        }

        ui.separator();

        // TODO: disable when nothing above
        if ui
            .add_enabled(true, Button::new((icons::ICON_NORTH, "Move up")))
            .clicked()
        {
            todo!();
            ui.close();
        }

        // TODO: disable when nothing below
        if ui
            .add_enabled(true, Button::new((icons::ICON_SOUTH, "Move down")))
            .clicked()
        {
            todo!();
            ui.close();
        }

        if ui
            .button((icons::ICON_DRIVE_FILE_RENAME, "Rename"))
            .clicked()
        {
            todo!();
            ui.close();
        }
    }

    /// Draws `ICON_MERGE` upside down (because there is no merge down material icon).
    fn draw_upside_down_merge_icon(
        ui: &Ui,
        rect: egui::Rect,
        font_id: FontId,
        color: egui::Color32,
    ) {
        if ui.is_rect_visible(rect) {
            let galley =
                ui.painter()
                    .layout_no_wrap(icons::ICON_MERGE.codepoint.into(), font_id, color);

            let pos = rect.center() + galley.size() * 0.5 + egui::vec2(0.0, 1.0);
            ui.painter().add(
                egui::epaint::TextShape::new(pos, galley, color).with_angle(std::f32::consts::PI),
            );
        }
    }
}

struct DrawLayerResults {
    selected: bool,
    delete_clicked: bool,
    interactable_hovered: bool,
}
