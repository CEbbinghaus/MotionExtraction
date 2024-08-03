@group(0)
@binding(0)
var<uniform> viewport: vec2<u32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec4<f32>) -> VertexOutput {
    var result: VertexOutput;
    result.position = position;
    return result;
}

@group(0)
@binding(1)
var cur_frame: texture_2d<u32>;

@group(0)
@binding(2)
var prev_frame: texture_2d<u32>;

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    let tex_position = vec2<i32>(i32(viewport.x) - i32(frag.position.x), i32(frag.position.y));
    let cur_col = vec4<f32>(textureLoad(cur_frame, vec2<i32>(tex_position.xy), 0)) / 256;
    let prev_col = vec4<f32>(textureLoad(prev_frame, vec2<i32>(tex_position.xy), 0)) / 256;
    return vec4<f32>(0.5 + ((cur_col.xyz * 0.5) - (prev_col.xyz * 0.5)), 1.0);
}
