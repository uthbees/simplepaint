struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var layer_texture: texture_2d<f32>;
@group(0) @binding(1) var layer_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Create an oversized triangle encompassing the canvas texture.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), // Bottom left
        vec2<f32>( 3.0, -1.0), // Far right
        vec2<f32>(-1.0,  3.0)  // Far top
    );

    // Uvs to map the vertices to the edges of the texture.
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0,  1.0), // Bottom left
        vec2<f32>(2.0,  1.0), // Far right
        vec2<f32>(0.0, -1.0)  // Far top
    );

    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Just sample the layer texture, the GPU alpha blending handles the rest automatically.
    return textureSample(layer_texture, layer_sampler, in.uv);
}
