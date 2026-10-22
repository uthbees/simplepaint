struct CameraUniform {
    pan: vec2<f32>,
    zoom: f32,
    _padding: f32,
    normalization_factors: vec2<f32>,
    _padding2: vec2<f32>,
};

@group(0) @binding(0) var canvas_texture: texture_2d<f32>;
@group(0) @binding(1) var canvas_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Create an oversized triangle encompassing the canvas panel.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), // Bottom left
        vec2<f32>( 3.0, -1.0), // Far right
        vec2<f32>(-1.0,  3.0)  // Far top
    );

    // Starting uv values. These have the canvas taking up the whole panel exactly, stretching if necessary.
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0,  1.0), // Bottom left
        vec2<f32>(2.0,  1.0), // Far right
        vec2<f32>(0.0, -1.0)  // Far top
    );

    let pos = positions[vertex_index];
    let starting_uv = uvs[vertex_index];

    out.position = vec4<f32>(pos, 0.0, 1.0);

    // First, center the uv on the origin and normalize the uv so that the canvas has the right aspect ratio and
    // a scale of one canvas pixel to one screen pixel.
    let normalized_uv = (starting_uv - 0.5) * camera.normalization_factors;
    // Then apply zoom and pan and re-center the uv on the center of the screen again.
    out.uv = (normalized_uv / camera.zoom) - camera.pan + 0.5;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.uv.x < 0.0 || in.uv.x > 1.0 || in.uv.y < 0.0 || in.uv.y > 1.0) {
        return vec4<f32>(0.15, 0.15, 0.15, 1.0);
    }

    return textureSample(canvas_texture, canvas_sampler, in.uv);
}
