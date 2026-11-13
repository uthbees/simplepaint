use crate::ActionRequestFlags;
use crate::app_state::canvas::{Canvas, LayerInitialPixels};
use crate::app_state::{AppState, MAX_CANVAS_HEIGHT, MAX_CANVAS_WIDTH};
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use arboard::Clipboard;
use egui::{MenuBar, Panel, Ui};
use image::{DynamicImage, GenericImageView, ImageBuffer};
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
                Self::draw_file_menu(ui, app_state, canvas_gpu, action_request_flags);
                Self::draw_edit_menu(ui, app_state, canvas_gpu);
            });
        });
    }

    fn draw_file_menu(
        ui: &mut Ui,
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
        action_request_flags: &mut ActionRequestFlags,
    ) {
        ui.menu_button("File", |ui| {
            if ui.button("Open").clicked()
                && let Some(file_path) = tinyfiledialogs::open_file_dialog(
                    "Select Image File",
                    "",
                    Some((&FILE_FILTER_PATTERNS, "Image Files")),
                )
            {
                Self::load_file_as_layer(app_state, canvas_gpu, &file_path);
                app_state.image_file_path = Some(file_path);
                app_state.canvas.changed_since_last_save = false;
            }
            if ui.button("Save").clicked() {
                if let Some(file_path) = &app_state.image_file_path {
                    let _ = Self::handle_save_to_file(&mut app_state.canvas, canvas_gpu, file_path);
                } else {
                    Self::handle_save_as(app_state, canvas_gpu);
                }
            }
            if ui.button("Save As").clicked() {
                Self::handle_save_as(app_state, canvas_gpu);
            }

            ui.separator();

            if ui.button("Quit").clicked() {
                action_request_flags.close_window = Some(());
            }
        });
    }

    fn draw_edit_menu(ui: &mut Ui, app_state: &mut AppState, canvas_gpu: &mut CanvasGpu) {
        ui.menu_button("Edit", |ui| {
            if ui.button("Copy").clicked() {
                todo!()
            }
            if ui.button("Paste").clicked()
                && let Some(image) =
                    Self::get_image_from_clipboard_or_display_error(app_state, canvas_gpu)
            {
                Self::add_layer_from_image(app_state, canvas_gpu, &image);
            }
        });
    }

    fn handle_save_as(app_state: &mut AppState, canvas_gpu: &mut CanvasGpu) {
        if let Some(file_path) = tinyfiledialogs::save_file_dialog_with_filter(
            "Save Image As",
            "",
            &FILE_FILTER_PATTERNS,
            "",
        ) && let Ok(()) =
            Self::handle_save_to_file(&mut app_state.canvas, canvas_gpu, file_path.clone())
        {
            app_state.image_file_path = Some(file_path);
        }
    }

    fn handle_save_to_file(
        canvas: &mut Canvas,
        canvas_gpu: &mut CanvasGpu,
        file_path: impl AsRef<Path>,
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
            canvas.changed_since_last_save = false;
            Ok(())
        }
    }

    fn load_file_as_layer(
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
        file_path: impl AsRef<Path>,
    ) {
        match image::open(file_path) {
            Ok(image) => Self::add_layer_from_image(app_state, canvas_gpu, &image),
            Err(err) => {
                MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_title("Failed to open image")
                    .set_description(format!("Failed to open selected image: {err}"))
                    .show();
            }
        }
    }

    fn add_layer_from_image(
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
        image: &DynamicImage,
    ) {
        let canvas = &mut app_state.canvas;

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
        if new_width != canvas.dimensions().width() || new_height != canvas.dimensions().height() {
            canvas.resize(canvas_gpu, DimensionsPxNonzero::new(new_width, new_height));
        }

        canvas.add_layer_specified(
            canvas_gpu,
            LayerInitialPixels::Tiles(Canvas::external_image_to_tiles(image), Color::TRANSPARENT),
            0,
            true,
        );
    }

    fn get_clipboard_or_display_error() -> Option<Clipboard> {
        match Clipboard::new() {
            Ok(clipboard) => Some(clipboard),
            Err(err) => {
                MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_title("Failed to set up clipboard")
                    .set_description(format!("Could not complete action: {err}"))
                    .show();
                None
            }
        }
    }

    fn get_image_from_clipboard_or_display_error(
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
    ) -> Option<DynamicImage> {
        if let Some(mut clipboard) = Self::get_clipboard_or_display_error() {
            if let Ok(image_data) = clipboard.get_image() {
                {
                    if let Some(image_buffer) = ImageBuffer::from_raw(
                        image_data.width as u32,
                        image_data.height as u32,
                        image_data.bytes.into_owned(),
                    ) {
                        return Some(DynamicImage::ImageRgba8(image_buffer));
                    }

                    MessageDialog::new()
                        .set_level(MessageLevel::Error)
                        .set_title("Paste failed")
                        .set_description(
                            "Could not decode clipboard image data (may be too big).".to_string(),
                        )
                        .show();
                }
            } else if let Ok(text) = clipboard.get_text() {
                let path = Path::new(&text);
                if path.try_exists().unwrap_or(false) {
                    Self::load_file_as_layer(app_state, canvas_gpu, path);
                }
            }
        }

        None
    }
}

// https://github.com/image-rs/image/blob/main/README.md#supported-image-formats
const FILE_FILTER_PATTERNS: [&str; 16] = [
    "*.avif", "*.bmp", "*.exr", "*.ff", "*.gif", "*.hdr", "*.ico", "*.jpg", "*.jpeg", "*.png",
    "*.pnm", "*.qoi", "*.tga", "*.tif", "*.tiff", "*.webp",
];
