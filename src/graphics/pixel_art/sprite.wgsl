// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv_mask: i32,
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
    @location(6) flip_mask: i32,
    @location(7) camera_locked: i32,
}

struct Camera {
    offset: vec2<f32>,
}
@group(2) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(
    in: VertexInput,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;

    let flip_h = instance.flip_mask % 2 == 1;
    let flip_v = instance.flip_mask >= 2;
    let uv_left = select(0.0, 1.0, flip_h);
    let uv_right = select(1.0, 0.0, flip_h);
    let uv_top = select(0.0, 1.0, flip_v);
    let uv_bottom = select(1.0, 0.0, flip_v);

    let uv_mask = in.uv_mask;
    switch uv_mask {
        case 1: {
            out.uv = vec2<f32>(uv_left, uv_top);
        }
        case 2: {
            out.uv = vec2<f32>(uv_right, uv_top);
        }
        case 4: {
            out.uv = vec2<f32>(uv_left, uv_bottom);
        }
        case 8: {
            out.uv = vec2<f32>(uv_right, uv_bottom);
        }
        default: {}
    }
    //out.uv = vec2<f32>(0.5);
    var world_pos = in.position.xy + w2p.offset + instance.offset.xy;
    // if we're not drawing camera locked ui
    if (instance.camera_locked == 0) {
        world_pos -= camera.offset;
    }
    out.clip_position = vec4<f32>(
        world_pos * w2p.scale,
        in.position.z + instance.offset.z,
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
    let color = textureSample(t_texture, t_sampler, in.uv);
    if (color.a == 0.0) {
        discard;
    }
    return color;
}