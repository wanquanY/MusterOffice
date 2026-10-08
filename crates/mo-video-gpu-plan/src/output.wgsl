@group(0) @binding(0) var<storage, read> source: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: ColorParams;
@vertex
fn full_frame(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
    let x = f32((id << 1u) & 2u);
    let y = f32(id & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}
@fragment
fn present_srgb(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<u32>(position.xy);
    return display_srgb(source[p.y * params.size.x + p.x], params);
}
