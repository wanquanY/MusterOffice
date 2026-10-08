// Shared color boundary math, f32 GPU counterpart of mo-color/space.rs and
// mo-video-render/sdr.rs. Extended signed sRGB is retained until SDR output.
struct ColorParams {
    size: vec4<u32>,
    row0: vec4<f32>,
    row1: vec4<f32>,
    row2: vec4<f32>,
    tone: vec4<f32>,
}
fn decode_srgb(v: vec3<f32>) -> vec3<f32> {
    let a = abs(v);
    return sign(v) * select(pow((a + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)), a / 12.92, a <= vec3<f32>(0.04045));
}
fn encode_srgb(v: vec3<f32>) -> vec3<f32> {
    let a = abs(v);
    return sign(v) * select(1.055 * pow(a, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055), 12.92 * a, a <= vec3<f32>(0.0031308));
}
fn primary_matrix(v: vec3<f32>, p: ColorParams) -> vec3<f32> {
    return vec3<f32>(dot(p.row0.xyz, v), dot(p.row1.xyz, v), dot(p.row2.xyz, v));
}
fn display_srgb(pixel: vec4<f32>, p: ColorParams) -> vec4<f32> {
    if pixel.a <= 0.0 { return vec4<f32>(0.0); }
    var rgb = pixel.rgb / pixel.a;
    if p.tone.w > 0.0 {
        let y = dot(p.tone.xyz, rgb);
        if y > 0.0 {
            let mapped = y * (1.0 + y / (p.tone.w * p.tone.w)) / (1.0 + y);
            rgb *= mapped / y;
        }
    }
    var encoded = clamp(encode_srgb(primary_matrix(rgb, p)), vec3<f32>(0.0), vec3<f32>(1.0));
    if p.size.z == 1u { encoded *= pixel.a; }
    return vec4<f32>(encoded, pixel.a);
}
