use std::num::NonZeroU32;
use std::sync::Arc;

use egui::ViewportId;
use egui_wgpu::winit::Painter;
use egui_wgpu::{RenderState, WgpuConfiguration};
use egui_winit::State as EguiWinitState;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::canvas::{Canvas, PixelColor};
use crate::gpu::CanvasGpu;
use crate::gui::{draw_canvas_panel, draw_side_panel};

/// Physical size of the drawing canvas in pixels.
const CANVAS_WIDTH: u32 = 1024;
const CANVAS_HEIGHT: u32 = 768;

const WINDOW_TITLE: &str = "SimplePaint";

pub struct App {
    /// CPU-side canvas buffer. Exists for the full lifetime of the app.
    canvas: Canvas,
    /// GPU/window state; `None` until `resumed()` fires and we have a window.
    window_state: Option<WindowState>,
    /// Whether the canvas CPU buffer has been modified since the last GPU upload.
    canvas_dirty: bool,
}

struct WindowState {
    window: Arc<Window>,
    painter: Painter,
    egui_state: EguiWinitState,
    render_state: RenderState,
    canvas_gpu: CanvasGpu,
}

impl App {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT),
            window_state: None,
            canvas_dirty: false,
        }
    }

    pub fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = self.window_state.as_mut() else {
            return;
        };

        let raw_input = state.egui_state.take_egui_input(&state.window);

        let egui_ctx = state.egui_state.egui_ctx().clone();

        let full_output = egui_ctx.run_ui(raw_input, |ui| {
            draw_side_panel(ui, egui::Color32::BLACK);

            let canvas_response = draw_canvas_panel(
                ui,
                Some(state.canvas_gpu.texture_id),
                self.canvas.width(),
                self.canvas.height(),
            );

            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss,
                clippy::cast_sign_loss
            )]
            // Handle canvas clicks.
            if let Some(response) = canvas_response
                && response.clicked()
                && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            {
                // The image rect in logical points.
                let rect = response.rect;

                // Convert logical-point position to canvas pixel index.
                let normalized_x = (pos.x - rect.min.x) / rect.width();
                let normalized_y = (pos.y - rect.min.y) / rect.height();
                let px = (normalized_x * self.canvas.width() as f32) as u32;
                let py = (normalized_y * self.canvas.height() as f32) as u32;

                self.canvas.set_pixel(px, py, PixelColor::BLACK);
                self.canvas_dirty = true;
            }
        });

        // Upload CPU canvas to GPU if it's changed.
        if self.canvas_dirty {
            state
                .canvas_gpu
                .upload(&state.render_state.queue, &self.canvas);
            self.canvas_dirty = false;
        }

        // Handle egui platform output (cursor, clipboard, etc.).
        state.egui_state.handle_platform_output_with_event_loop(
            &state.window,
            event_loop,
            full_output.platform_output,
        );

        let pixels_per_point = egui_ctx.pixels_per_point();
        let clipped_primitives = egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        let mut textures_delta = full_output.textures_delta;

        state.painter.paint_and_update_textures(
            ViewportId::ROOT,
            pixels_per_point,
            [0.1, 0.1, 0.1, 1.0],
            &clipped_primitives,
            &mut textures_delta,
            vec![],
            &state.window,
        );

        // Make the window visible, if this is the first render.
        if !state.window.is_visible().unwrap_or(true) {
            state.window.set_visible(true);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize the window only once.
        if self.window_state.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_visible(false);
        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        // Build egui context and winit state.
        let viewport_id = ViewportId::ROOT;
        #[allow(clippy::cast_possible_truncation)]
        let egui_state = EguiWinitState::new(
            egui::Context::default(),
            viewport_id,
            event_loop,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );

        // Initialize the egui-wgpu Painter. (This creates a wgpu instance.)
        let mut painter = pollster::block_on(Painter::new(
            egui::Context::default(),
            WgpuConfiguration::default(),
            false,
            egui_wgpu::RendererOptions::default(),
        ));

        // Attach the window surface and initialize the device/queue.
        pollster::block_on(painter.set_window(viewport_id, Some(Arc::clone(&window))))
            .expect("Failed to set painter window");

        let render_state = painter
            .render_state()
            .expect("RenderState should be available after set_window");

        // Upload the initial white canvas to a wgpu texture.
        let canvas_gpu = CanvasGpu::new(
            &render_state,
            self.canvas.width(),
            self.canvas.height(),
            self.canvas.as_bytes(),
        );

        self.window_state = Some(WindowState {
            window,
            painter,
            egui_state,
            render_state,
            canvas_gpu,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.window_state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                self.render_frame(event_loop);
                return;
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(w), Some(h)) = (
                    NonZeroU32::new(new_size.width),
                    NonZeroU32::new(new_size.height),
                ) {
                    state.painter.on_window_resized(ViewportId::ROOT, w, h);
                    self.render_frame(event_loop);
                }
                return;
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            _ => {}
        }

        // Forward all other events to egui.
        let egui_response = state.egui_state.on_window_event(&state.window, &event);
        if egui_response.repaint {
            state.window.request_redraw();
        }
    }
}
