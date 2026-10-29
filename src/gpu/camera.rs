use crate::app_state::AppState;
use bytemuck::{Pod, Zeroable};
use wgpu::BindingResource;
use wgpu::util::DeviceExt;

/// Camera parameters passed to the canvas shader.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    /// Pan amount in a percentage of the canvas panned (with the origin being the center of the canvas).
    /// For example, if the bottom right corner of the canvas was in the center of the screen,
    /// `pan` would be [0.5, 0.5].
    pan: [f32; 2],
    zoom: f32,
    _padding: f32,
    /// Factors to multiply with the starting uv to normalize the canvas to the proper aspect ratio
    /// and a 1:1 canvas pixel:screen pixel scale.
    normalization_factors: [f32; 2],
    _padding2: [f32; 2],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            pan: [0.0, 0.0],
            zoom: 1.0,
            _padding: 0.0,
            normalization_factors: [1.0, 1.0],
            _padding2: [0.0, 0.0],
        }
    }
}

pub struct Camera {
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
}

impl Camera {
    pub fn new(device: &wgpu::Device) -> Self {
        let camera_uniform = CameraUniform::default();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            uniform: camera_uniform,
            buffer: camera_buffer,
        }
    }

    pub fn get_buffer_binding(&self) -> BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    /// Updates the `CameraUniform` and uploads the updated data.
    /// Returns the computed aspect ratio correction.
    pub fn update(&mut self, queue: &wgpu::Queue, app_state: &AppState) -> [f32; 2] {
        let canvas_dim = app_state.canvas.dimensions();
        let pan_px = app_state.canvas_view.pan_px();

        self.uniform = CameraUniform {
            pan: [
                pan_px.x / canvas_dim.width() as f32,
                pan_px.y / canvas_dim.height() as f32,
            ],
            zoom: app_state.canvas_view.zoom(),
            _padding: 0.0,
            normalization_factors: app_state
                .canvas_view
                .get_normalization_factors(&app_state.canvas),
            _padding2: [0.0, 0.0],
        };

        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.uniform));

        [1.0, 1.0]
    }
}
