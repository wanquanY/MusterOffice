//! IEC sRGB transfer and HSL arithmetic, with one portable libm path on all targets.
pub fn srgb_to_linear(v: f64) -> f64 {
    let a = v.abs();
    let linear = if a <= 0.04045 {
        a / 12.92
    } else {
        libm::pow((a + 0.055) / 1.055, 2.4)
    };
    linear.copysign(v)
}
pub fn linear_to_srgb(v: f64) -> f64 {
    let a = v.abs();
    let encoded = if a <= 0.0031308 {
        12.92 * a
    } else {
        1.055 * libm::pow(a, 1.0 / 2.4) - 0.055
    };
    encoded.copysign(v)
}
pub(super) fn srgb_to_hsl(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let luminance = (max + min) * 0.5;
    if delta == 0.0 {
        return [0.0, 0.0, luminance];
    }
    let saturation = delta / (1.0 - (2.0 * luminance - 1.0).abs());
    let hue = if max == r {
        (g - b) / delta
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    [super::wrap(hue / 6.0), saturation, luminance]
}
pub(super) fn hsl_to_srgb(hsl: [f64; 3]) -> [f64; 3] {
    let [h, s, l] = hsl;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let sector = h * 6.0;
    let x = c * (1.0 - (sector - 2.0 * libm::floor(sector / 2.0) - 1.0).abs());
    let rgb = match sector as u8 {
        0 => [c, x, 0.0],
        1 => [x, c, 0.0],
        2 => [0.0, c, x],
        3 => [0.0, x, c],
        4 => [x, 0.0, c],
        _ => [c, 0.0, x],
    };
    rgb.map(|v| v + l - c * 0.5)
}
