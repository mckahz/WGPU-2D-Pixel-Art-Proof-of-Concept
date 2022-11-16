// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
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

struct TileInstance {
    @location(5) offset: vec2<f32>,
    @location(6) flip_mask: i32,
    @location(7) tile_index: i32,
}

struct Camera {
    offset: vec2<f32>,
}
@group(2) @binding(0)
var<uniform> camera: Camera;

@group(3) @binding(0)
var<uniform> depth: f32;

@vertex
fn vs_main(
    in: VertexInput,
    instance: TileInstance,
) -> VertexOutput {
    var out: VertexOutput;
    let width = 16.0;
    let height = 5.0;

    let unflipped_left = (f32(instance.tile_index) % width) / width;
    let unflipped_right = 1.0 / width + unflipped_left;
    let unflipped_top = (floor(f32(instance.tile_index) / width)) / height;
    let unflipped_bottom = 1.0 / height + unflipped_top;

    let flip_h = instance.flip_mask % 2 == 1;
    let flip_v = instance.flip_mask >= 2;
    let uv_left = select(unflipped_left, unflipped_right, flip_h);
    let uv_right = select(unflipped_right, unflipped_left, flip_h);
    let uv_top = select(unflipped_top, unflipped_bottom, flip_v);
    let uv_bottom = select(unflipped_bottom, unflipped_top, flip_v);

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
    let world_pos = in.position.xy + w2p.offset + instance.offset.xy;
    out.clip_position = vec4<f32>(
        (world_pos - camera.offset) * w2p.scale,
        depth,
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