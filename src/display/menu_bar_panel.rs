use crate::ActionRequestFlags;
use crate::app_state::canvas::{Canvas, LayerInitialPixels};
use crate::app_state::{AppState, MAX_CANVAS_HEIGHT, MAX_CANVAS_WIDTH};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui::{MenuBar, Panel, Ui};
use image::GenericImageView;
use rfd::{MessageDialog, MessageLevel};
use std::error::Error;
use std::path::Path;
use tiny_skia::Color;

pub struct MenuBarPanel {}

impl MenuBarPanel {
    pub fn draw(
        ui: &mut Ui,
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
        action_request_flags: &mut ActionRequestFlags,
    ) {
        Panel::top("menu_bar_panel").show(ui, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked()
                        && let Some(file_path) = tinyfiledialogs::open_file_dialog(
                            "Select Image File",
                            "",
                            Some((&FILE_FILTER_PATTERNS, "Image Files")),
                        )
                    {
                        Self::load_file_as_layer(app_state, &file_path, canvas_gpu);
                        app_state.image_file_path = Some(file_path);
                        action_request_flags.handle_file_changed = Some(());
                        action_request_flags.set_canvas_saved = Some(true);
                    }
                    if ui.button("Save").clicked() {
                        if let Some(file_path) = &app_state.image_file_path {
                            let _ = Self::handle_save_to_file(
                                canvas_gpu,
                                file_path,
                                action_request_flags,
                            );
                        } else {
                            Self::handle_save_as(app_state, canvas_gpu, action_request_flags);
                        }
                    }
                    if ui.button("Save As").clicked() {
                        Self::handle_save_as(app_state, canvas_gpu, action_request_flags);
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        action_request_flags.close_window = Some(());
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Copy").clicked() {
                        todo!()
                    }
                    if ui.button("Paste").clicked() {
                        todo!()
                    }
                });
            });
        });
    }

    fn handle_save_as(
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
        action_request_flags: &mut ActionRequestFlags,
    ) {
        if let Some(file_path) = tinyfiledialogs::save_file_dialog_with_filter(
            "Save Image As",
            "",
            &FILE_FILTER_PATTERNS,
            "",
        ) && let Ok(()) =
            Self::handle_save_to_file(canvas_gpu, file_path.clone(), action_request_flags)
        {
            app_state.image_file_path = Some(file_path);
        }
    }

    fn handle_save_to_file(
        canvas_gpu: &mut CanvasGpu,
        file_path: impl AsRef<Path>,
        action_request_flags: &mut ActionRequestFlags,
    ) -> Result<(), Box<dyn Error>> {
        let result = canvas_gpu
            .composite_canvas()
            .save_texture_to_file(file_path);
        if let Err(err) = result {
            MessageDialog::new()
                .set_level(MessageLevel::Error)
                .set_title("Failed to save image")
                .set_description(format!("Failed to save image: {err}"))
                .show();
            Err(err)
        } else {
            action_request_flags.set_canvas_saved = Some(true);
            Ok(())
        }
    }

    fn load_file_as_layer(app_state: &mut AppState, file_path: &str, canvas_gpu: &mut CanvasGpu) {
        let canvas = &mut app_state.canvas;

        match image::open(file_path) {
            Ok(image) => {
                let (width, height) = image.dimensions();
                if width > MAX_CANVAS_WIDTH || height > MAX_CANVAS_HEIGHT {
                    MessageDialog::new()
                        .set_level(MessageLevel::Error)
                        .set_title("Failed to load image")
                        .set_description(format!(
                            "Cannot load images larger than {MAX_CANVAS_WIDTH}x{MAX_CANVAS_HEIGHT}px\
                             (selected image was {width}x{height}px)"
                        ))
                        .show();
                    return;
                }

                let new_width = canvas.dimensions().width().max(width);
                let new_height = canvas.dimensions().height().max(height);
                if new_width != canvas.dimensions().width()
                    || new_height != canvas.dimensions().height()
                {
                    canvas.resize(DimensionsPxNonzero::new(new_width, new_height), canvas_gpu);
                }

                canvas.add_layer_specified(
                    canvas_gpu,
                    LayerInitialPixels::Tiles(
                        Canvas::external_image_to_tiles(&image),
                        Color::TRANSPARENT,
                    ),
                    0,
                    true,
                );
            }
            Err(err) => {
                MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_title("Failed to open image")
                    .set_description(format!("Failed to open selected image: {err}"))
                    .show();
            }
        }
    }
}

pub struct SpecialActions {
    pub mark_saved: Box<dyn FnMut()>,
    pub request_window_close: Box<dyn FnMut()>,
    pub handle_file_changed: Box<dyn FnMut()>,
}

// https://github.com/image-rs/image/blob/main/README.md#supported-image-formats
const FILE_FILTER_PATTERNS: [&str; 16] = [
    "*.avif", "*.bmp", "*.exr", "*.ff", "*.gif", "*.hdr", "*.ico", "*.jpg", "*.jpeg", "*.png",
    "*.pnm", "*.qoi", "*.tga", "*.tif", "*.tiff", "*.webp",
];
