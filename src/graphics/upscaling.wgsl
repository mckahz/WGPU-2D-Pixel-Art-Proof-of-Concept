// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct WorldToPixel {
    scale: vec2<f32>,
    offset: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> w2p: WorldToPixel;

struct Instance {
    @location(5) offset: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = model.uv;
    out.clip_position = vec4<f32>(
        (model.position.xy + w2p.offset + instance.offset.xy) * w2p.scale,
        model.position.z + instance.offset.z,
        1.0
    );
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var t_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, t_sampler, in.uv);
}