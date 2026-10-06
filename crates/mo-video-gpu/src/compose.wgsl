// Original linear-light compositor. Raster coefficients are signed Q32.32;
// source positions are Q30.33. Two u32 words avoid float boundary decisions
// and do not require an optional shader i64 feature.
struct Params {
    sizes: vec4<u32>,
    row_x: vec4<u32>,
    row_y: vec4<u32>,
    offset: vec4<u32>,
    values: vec4<f32>,
    solid: vec4<f32>,
    options: vec4<u32>,
}
@group(0) @binding(0) var<storage, read> source: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: Params;

fn add64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let low = a.x + b.x;
    return vec2<u32>(low, a.y + b.y + select(0u, 1u, low < a.x));
}
// Multiplier <= 32767. Each 16-bit partial product fits u32; modular
// two's-complement multiplication also handles negative coefficients.
fn mul64(a: vec2<u32>, b: u32) -> vec2<u32> {
    let p0 = (a.x & 65535u) * b;
    let p1 = (a.x >> 16u) * b;
    let low = p0 + (p1 << 16u);
    return vec2<u32>(low, a.y * b + (p1 >> 16u) + select(0u, 1u, low < p0));
}
fn coordinate(row: vec4<u32>, offset: vec2<u32>, position: vec2<u32>) -> vec2<u32> {
    return add64(add64(mul64(row.xy, position.x), mul64(row.zw, position.y)), mul64(offset, 2u));
}
// Remove the pixel-center half, clamp exactly, then convert only the local
// fraction. Large image coordinates never lose fractional precision in f32.
fn bilinear_axis(q: vec2<u32>, extent: u32) -> vec2<u32> {
    if q.y == 0u { return vec2<u32>(0u); }
    let shifted = vec2<u32>(q.x, q.y - 1u);
    if shifted.y >= (extent - 1u)*2u { return vec2<u32>(0u, (extent - 1u)*2u); }
    return shifted;
}
fn fraction(q: vec2<u32>) -> f32 {
    return f32(q.y & 1u)*0.5 + f32(q.x)/8589934592.0;
}
fn read_source(position: vec2<u32>) -> vec4<f32> {
    return source[position.y * params.sizes.z + position.x];
}
fn sample_source(qx: vec2<u32>, qy: vec2<u32>) -> vec4<f32> {
    if params.options.y == 0u { return read_source(vec2<u32>(qx.y >> 1u, qy.y >> 1u)); }
    let x = bilinear_axis(qx, params.sizes.z);
    let y = bilinear_axis(qy, params.sizes.w);
    let start = vec2<u32>(x.y >> 1u, y.y >> 1u);
    let end = min(start + vec2<u32>(1u), params.sizes.zw - vec2<u32>(1u));
    let upper = mix(read_source(start), read_source(vec2<u32>(end.x, start.y)), fraction(x));
    let lower = mix(read_source(vec2<u32>(start.x, end.y)), read_source(end), fraction(x));
    return mix(upper, lower, fraction(y));
}
@compute @workgroup_size(8, 8)
fn compose(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.sizes.x || id.y >= params.sizes.y { return; }
    let p = id.xy * 2u + vec2<u32>(1u);
    let qx = coordinate(params.row_x, params.offset.xy, p);
    let qy = coordinate(params.row_y, params.offset.zw, p);
    // Negative coordinates have the sign bit set and also exceed this bound.
    if qx.y >= params.sizes.z*2u || qy.y >= params.sizes.w*2u { return; }
    var color = params.solid;
    if params.options.x == 0u { color = sample_source(qx, qy); }
    let src = color * params.values.x;
    let index = id.y * params.sizes.x + id.x;
    output_pixels[index] = src + output_pixels[index] * (1.0 - src.a);
}
