use crate::ActionRequestFlags;
use crate::app_state::canvas::{Canvas, LayerInitialPixels};
use crate::app_state::{AppState, MAX_CANVAS_HEIGHT, MAX_CANVAS_WIDTH};
use crate::gpu::CanvasGpu;
use crate::utils::Utils;
use crate::utils::dimensions::DimensionsPxNonzero;
use arboard::Clipboard;
use egui::{MenuBar, Panel, Ui};
use image::{DynamicImage, GenericImageView, ImageBuffer, RgbaImage, imageops};
use std::borrow::Cow;
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
                    Some((&LOAD_FILE_FILTER_PATTERNS, "Image Files")),
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
                Self::handle_copy(canvas_gpu);
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
            &SAVE_FILE_FILTER_PATTERNS,
            "",
        ) {
            // Default to PNG if no format was specified.
            let mut file_path = file_path.trim_end_matches('.').to_owned();
            if !file_path.contains('.') {
                file_path += ".png";
            }

            if let Ok(()) = Self::handle_save_to_file(&mut app_state.canvas, canvas_gpu, &file_path)
            {
                app_state.image_file_path = Some(file_path);
            }
        }
    }

    fn handle_save_to_file(
        canvas: &mut Canvas,
        canvas_gpu: &mut CanvasGpu,
        file_path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        match canvas_gpu.composite_canvas().export_texture_to_image() {
            Ok(image) => {
                let image = DynamicImage::from(image);

                let is_jpg = file_path
                    .as_ref()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg")
                    });

                let save_result = if is_jpg {
                    DynamicImage::ImageRgb8(Self::flatten_img_onto_white(&image)).save(file_path)
                } else {
                    image.save(file_path)
                };

                match save_result {
                    Ok(()) => {
                        canvas.changed_since_last_save = false;
                        Ok(())
                    }
                    Err(err) => {
                        Utils::show_error_dialog(
                            "Failed to save image",
                            &format!("Failed to save image: {err}"),
                        );
                        Err(err.into())
                    }
                }
            }
            Err(err) => {
                Utils::show_error_dialog(
                    "Failed to save image",
                    &format!("Failed to save image: {err}"),
                );
                Err(err)
            }
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
                Utils::show_error_dialog(
                    "Failed to open image",
                    &format!("Failed to open selected image: {err}"),
                );
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
            Utils::show_error_dialog(
                "Failed to load image",
                &format!(
                    "Cannot load images larger than {MAX_CANVAS_WIDTH}x{MAX_CANVAS_HEIGHT}px\
                             (selected image was {width}x{height}px)"
                ),
            );
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

    fn handle_copy(canvas_gpu: &mut CanvasGpu) {
        if let Some(mut clipboard) = Self::get_clipboard_or_display_error() {
            match canvas_gpu.composite_canvas().export_texture_to_image() {
                Ok(image) => {
                    let width = image.width() as usize;
                    let height = image.height() as usize;
                    let bytes = image.into_raw();

                    let image_data = arboard::ImageData {
                        width,
                        height,
                        bytes: Cow::Owned(bytes),
                    };

                    if let Err(err) = clipboard.set_image(image_data) {
                        Utils::show_error_dialog(
                            "Failed to copy image",
                            &format!("Could not set clipboard: {err}"),
                        );
                    }
                }
                Err(err) => {
                    Utils::show_error_dialog(
                        "Failed to copy image",
                        &format!("Could not export image to clipboard: {err}"),
                    );
                }
            }
        }
    }

    fn get_clipboard_or_display_error() -> Option<Clipboard> {
        match Clipboard::new() {
            Ok(clipboard) => Some(clipboard),
            Err(err) => {
                Utils::show_error_dialog(
                    "Failed to set up clipboard",
                    &format!("Could not complete action: {err}"),
                );
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

                    Utils::show_error_dialog(
                        "Paste failed",
                        "Could not decode clipboard image data (may be too big).",
                    );
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

    fn flatten_img_onto_white(img: &DynamicImage) -> image::RgbImage {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut canvas: RgbaImage =
            ImageBuffer::from_pixel(width, height, image::Rgba([255, 255, 255, 255]));
        imageops::overlay(&mut canvas, &rgba, 0, 0);
        DynamicImage::ImageRgba8(canvas).to_rgb8()
    }
}

// https://github.com/image-rs/image/blob/main/README.md#supported-image-formats
// Excluded because of potential format problems that would need to be tested/handled individually:
// "*.gif"
const LOAD_FILE_FILTER_PATTERNS: [&str; 15] = [
    "*.jpg", "*.jpeg", "*.png", "*.webp", "*.avif", "*.bmp", "*.ff", "*.qoi", "*.tga", "*.tif",
    "*.tiff", "*.hdr", "*.ico", "*.pnm", "*.exr",
];
// Excluded because of potential format problems that would need to be tested/handled individually:
// "*.hdr", "*.ico", "*.gif", "*.exr"
const SAVE_FILE_FILTER_PATTERNS: [&str; 12] = [
    "*.jpg", "*.jpeg", "*.png", "*.webp", "*.avif", "*.bmp", "*.ff", "*.qoi", "*.tga", "*.tif",
    "*.tiff", "*.pnm",
];
