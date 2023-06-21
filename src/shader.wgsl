// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) tex_idx: u32,
};

struct InstanceInput {
    @location(3) pos: vec2<f32>,
    @location(4) size: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tex_idx: u32,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.tex_idx = model.tex_idx;
    out.clip_position = vec4<f32>(instance.pos, 0.0, 0.0) + vec4<f32>(model.position, 1.0) * vec4<f32>(instance.size, 1.0, 1.0);
    return out;
}

// Fragment shader

@group(0)@binding(0)
var d0_t: texture_2d<f32>;
@group(0)@binding(1)
var d0_s: sampler;

@group(1)@binding(0)
var d1_t: texture_2d<f32>;
@group(1)@binding(1)
var d1_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var t0c = textureSample(d0_t, d0_s, in.tex_coords);
    var t1c = textureSample(d1_t, d1_s, in.tex_coords);
    return select(t0c, t1c, in.tex_idx > 0u);
}
