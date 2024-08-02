// @group(0) @binding(0)
// var current_frame_texture : texture_2d<f32>;
// @group(0) @binding(1)
// var prev_frame_texture : texture_2d<f32>;
// @group(0) @binding(2)
// var output_texture : texture_storage_2d<rgba8unorm, write>;

// @compute
// @workgroup_size(1)
// fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
//     let dimensions = textureDimensions(current_frame_texture);
//     let prev_dimmensions = textureDimensions(prev_frame_texture);
//     let coords = vec2<i32>(global_id.xy);

//     let xy = vec2<f32>(coords) / vec2<f32>(dimensions);

//     // if(coords.x >= dimensions.x || coords.y >= dimensions.y) {
//     //     return;
//     // }

//     // let color = textureLoad(current_frame_texture, coords.xy, 0);
//     // let gray = dot(vec3<f32>(0.299, 0.587, 0.114), color.rgb);

//     textureStore(output_texture, coords.xy, vec4<f32>(xy.x, xy.y, 1.0, 1.0));
// }

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
    let uv = frag.position.xy / vec2<f32>(viewport);
    let cur_col = vec4<f32>(textureLoad(cur_frame, vec2<i32>(frag.position.xy), 0)) / 256;
    let prev_col = vec4<f32>(textureLoad(prev_frame, vec2<i32>(frag.position.xy), 0)) / 256;
    return vec4<f32>(0.5 + ((cur_col.xyz * 0.5) - (prev_col.xyz * 0.5)), 1.0);
}
