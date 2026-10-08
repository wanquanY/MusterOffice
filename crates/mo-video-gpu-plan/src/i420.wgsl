// Explicit BT.709 / ideal BT.1886, limited range, left-cosited Lanczos2 I420.
// Each packed word has a single writer. All full-image temporaries are bounded
// by the host executor; padding bytes remain zero.
struct I420Params {
    color: ColorParams,
    matte: vec4<f32>,
    horizontal: array<vec4<f32>, 2>,
    vertical: array<vec4<f32>, 2>,
    planes: vec4<u32>,
}
@group(0) @binding(0) var<uniform> output_params: I420Params;
@group(0) @binding(1) var<storage, read> linear_pixels: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> television: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> horizontal_chroma: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read_write> packed: array<u32>;

@compute @workgroup_size(8, 8)
fn television_signal(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = output_params.color;
    if id.x >= p.size.x || id.y >= p.size.y { return; }
    let index = id.y * p.size.x + id.x;
    let pixel = linear_pixels[index];
    var rgb = pixel.rgb + (1.0 - pixel.a) * output_params.matte.rgb;
    if p.tone.w > 0.0 {
        let y = dot(p.tone.xyz, rgb);
        if y > 0.0 {
            rgb *= (1.0 + y / (p.tone.w * p.tone.w)) / (1.0 + y);
        }
    }
    rgb = pow(clamp(primary_matrix(rgb, p), vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.4));
    let y = dot(vec3<f32>(0.2126, 0.7152, 0.0722), rgb);
    television[index] = vec4<f32>(y, (rgb.b - y) / (2.0 * (1.0 - 0.0722)), (rgb.r - y) / (2.0 * (1.0 - 0.2126)), 0.0);
}
@compute @workgroup_size(8, 8)
fn chroma_horizontal(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = output_params.color.size.x;
    let half = width / 2u;
    if id.x >= half || id.y >= output_params.color.size.y { return; }
    let x = id.x;
    let y = id.y;
    var chroma = vec2<f32>(0.0);
    for (var i = 0u; i < 8u; i++) {
        let source_x = u32(clamp(i32(2u * x + i) - 3, 0, i32(width) - 1));
        chroma += television[y * width + source_x].yz * output_params.horizontal[i / 4u][i % 4u];
    }
    horizontal_chroma[y * half + x] = chroma;
}
fn chroma_sample(x: u32, y: u32, channel: u32) -> f32 {
    let width = output_params.color.size.x;
    let height = output_params.color.size.y;
    var value = 0.0;
    for (var i = 0u; i < 8u; i++) {
        let source_y = u32(clamp(i32(2u * y + i) - 3, 0, i32(height) - 1));
        value += horizontal_chroma[source_y * (width / 2u) + x][channel] * output_params.vertical[i / 4u][i % 4u];
    }
    return floor(clamp(128.0 + 224.0 * value, 16.0, 240.0) + 0.5);
}
@compute @workgroup_size(8, 8)
fn pack_i420(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = output_params.color.size.x;
    let height = output_params.color.size.y;
    let planes = output_params.planes;
    if id.y >= height * 2u { return; }
    let is_y = id.y < height;
    let channel = select(0u, 1u, id.y >= height + height / 2u);
    let stride = select(planes.y, planes.x, is_y);
    if id.x * 4u >= stride { return; }
    let row = select(id.y - height - channel * (height / 2u), id.y, is_y);
    let base = select(select(planes.z, planes.w, channel == 1u), 0u, is_y);
    let offset = base + row * stride + id.x * 4u;
    var result = 0u;
    for (var i = 0u; i < 4u; i++) {
        var value = 0u;
        let x = id.x * 4u + i;
        if is_y {
            if x < width {
                value = u32(floor(clamp(16.0 + 219.0 * television[row * width + x].x, 16.0, 235.0) + 0.5));
            }
        } else {
            if x < width / 2u { value = u32(chroma_sample(x, row, channel)); }
        }
        result |= value << (i * 8u);
    }
    packed[offset / 4u] = result;
}
