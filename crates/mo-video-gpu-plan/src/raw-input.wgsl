// Counterpart of shared CPU raw-plane decoding. No implicit platform EOTF,
// chroma siting, clamp or alpha conversion occurs before these operations.
struct InputParams {
    size: vec4<u32>, sampling: vec4<u32>,
    plane0: vec4<u32>, plane1: vec4<u32>, plane2: vec4<u32>, plane3: vec4<u32>,
    order: vec4<u32>, row0: vec4<f32>, row1: vec4<f32>, row2: vec4<f32>,
    signal: vec4<f32>, chroma: vec4<f32>, transfer: vec4<f32>, luma: vec4<f32>,
}
@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> p: InputParams;
@group(0) @binding(3) var<storage, read_write> invalid_samples: atomic<u32>;
fn byte_at(offset: u32) -> u32 { return (source[offset / 4u] >> ((offset % 4u) * 8u)) & 255u; }
fn code(plane: vec4<u32>, xy: vec2<u32>, channel: u32) -> f32 {
    let at = plane.x + xy.y * plane.y + (xy.x * plane.z + channel) * p.sampling.z;
    var value = byte_at(at);
    if p.sampling.z == 2u { value |= byte_at(at + 1u) << 8u; }
    if value >= (1u << p.size.w) { atomicOr(&invalid_samples,1u); }
    return f32(value);
}
fn chroma_code(plane: vec4<u32>, xy: vec2<f32>, channel: u32) -> f32 {
    let size = (p.size.xy + p.sampling.xy - vec2<u32>(1u)) / p.sampling.xy;
    let at = clamp(xy, vec2<f32>(0.0), vec2<f32>(size - vec2<u32>(1u)));
    let lo = vec2<u32>(floor(at)); let hi = min(lo + vec2<u32>(1u), size - vec2<u32>(1u));
    let frac = at - vec2<f32>(lo);
    return mix(mix(code(plane,lo,channel),code(plane,vec2<u32>(hi.x,lo.y),channel),frac.x),
        mix(code(plane,vec2<u32>(lo.x,hi.y),channel),code(plane,hi,channel),frac.x),frac.y);
}
fn eotf(rgb: vec3<f32>) -> vec3<f32> {
    let kind = u32(p.transfer.x);
    if kind == 0u { return rgb; }
    if kind == 1u {
        let v = abs(rgb);
        return sign(rgb)*select(pow((v+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),v/12.92,v<=vec3<f32>(0.04045));
    }
    if kind == 2u { return pow(max(rgb,vec3<f32>(0.0)),vec3<f32>(2.4)); }
    if kind == 3u || kind == 4u {return sign(rgb)*pow(abs(rgb),vec3<f32>(select(2.2,2.8,kind==4u)));}
    let v = clamp(rgb,vec3<f32>(0.0),vec3<f32>(1.0));
    if kind == 5u {
        let q = pow(v,vec3<f32>(32.0/2523.0));
        return (10000.0/p.transfer.w)*pow(max(q-vec3<f32>(3424.0/4096.0),vec3<f32>(0.0))/(vec3<f32>(2413.0/128.0)-(2392.0/128.0)*q),vec3<f32>(16384.0/2610.0));
    }
    let a = 0.17883277; let b = 1.0 - 4.0*a; let c = 0.5-a*log(4.0*a);
    let scene = select((exp((v-vec3<f32>(c))/a)+vec3<f32>(b))/12.0,v*v/3.0,v<=vec3<f32>(0.5));
    let y = max(dot(p.luma.xyz,scene),0.0);
    if y == 0.0 {return vec3<f32>(0.0);}
    return scene*(pow(y,p.transfer.z-1.0)*p.transfer.y/p.transfer.w);
}
@compute @workgroup_size(8,8)
fn normalize(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.size.x || id.y >= p.size.y { return; }
    let maxcode = f32((1u << p.size.w)-1u);
    var alpha = 1.0; var rgb = vec3<f32>(0.0);
    if p.size.z == 0u {
        rgb = (vec3<f32>(code(p.plane0,id.xy,p.order.x),code(p.plane0,id.xy,p.order.y),code(p.plane0,id.xy,p.order.z))-vec3<f32>(p.signal.x))*p.signal.y;
        if (p.sampling.w & 1u) == 1u { alpha = code(p.plane0,id.xy,p.order.w)/maxcode; }
    } else {
        let xy = (vec2<f32>(id.xy)-p.chroma.xy)/vec2<f32>(p.sampling.xy);
        let y = (code(p.plane0,id.xy,0u)-p.signal.x)*p.signal.y;
        let cb = (chroma_code(p.plane1,xy,0u)-p.signal.z)*p.signal.w;
        var cr = 0.0;
        if (p.sampling.w & 256u) == 256u { cr=(chroma_code(p.plane1,xy,1u)-p.signal.z)*p.signal.w; }
        else { cr=(chroma_code(p.plane2,xy,0u)-p.signal.z)*p.signal.w; }
        let kr=p.chroma.z;let kb=p.chroma.w;
        let r=y+2.0*(1.0-kr)*cr;let b=y+2.0*(1.0-kb)*cb;
        rgb=vec3<f32>(r,(y-kr*r-kb*b)/(1.0-kr-kb),b);
        if (p.sampling.w & 1u) == 1u { alpha=code(p.plane3,id.xy,0u)/maxcode; }
    }
    var result=vec4<f32>(0.0);
    if alpha > 0.0 {
        let linear=eotf(rgb);
        result=vec4<f32>(vec3<f32>(dot(p.row0.xyz,linear),dot(p.row1.xyz,linear),dot(p.row2.xyz,linear))*alpha,alpha);
    }
    output_pixels[id.y*p.size.x+id.x]=result;
}
