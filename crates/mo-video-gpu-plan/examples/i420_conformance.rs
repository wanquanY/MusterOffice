//! CPU reference for actual browser GPU television output, including padding,
//! transparent matte, HDR, boundary filtering and fractional byte rounding.
use mo_video_gpu_plan::I420Parameters;
use mo_video_render::{I420Options, LinearImage, PixelLimits, ToneMap};
use serde_json::json;

fn main() {
    let mut cases = Vec::new();
    for (width, height) in [(2, 2), (6, 4), (18, 10), (64, 32), (128, 96)] {
        for (name, hdr, peak, matte) in [
            ("transparent-black", false, None, [0, 0, 0]),
            ("transparent-white", false, None, [255, 255, 255]),
            ("gradient", false, None, [24, 24, 24]),
            ("chroma-boundaries", false, None, [10, 110, 220]),
            ("hdr", true, Some(1000.), [24, 24, 24]),
        ] {
            let mut pixels = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let alpha = ((x + y * 3) % 17) as f32 / 16.;
                    let rgb = if name.starts_with("transparent") {
                        [0.; 3]
                    } else if name == "chroma-boundaries" {
                        match (x + y) % 3 {
                            0 => [1., 0., 0.],
                            1 => [0., 1., 0.],
                            _ => [0., 0., 1.],
                        }
                    } else {
                        [
                            x as f32 / width as f32,
                            y as f32 / height as f32,
                            ((x * 13 + y * 7) % 97) as f32 / 96.,
                        ]
                    };
                    let alpha = if name.starts_with("transparent") {
                        0.
                    } else {
                        alpha
                    };
                    let power = if hdr { 4. } else { 1. };
                    pixels.push([
                        rgb[0] * power * alpha,
                        rgb[1] * power * alpha,
                        rgb[2] * power * alpha,
                        alpha,
                    ]);
                }
            }
            let tone =
                peak.map(|source_peak_nits| ToneMap::ReinhardLuminanceV1 { source_peak_nits });
            let image = LinearImage::from_pixels(
                width,
                height,
                pixels,
                hdr,
                PixelLimits::default(),
                &|| false,
            )
            .unwrap();
            let result = image
                .to_i420(
                    I420Options {
                        background_srgb: matte,
                        tone_map: tone,
                        max_bytes: 1 << 20,
                        max_temporary_bytes: 1 << 20,
                    },
                    &|| false,
                )
                .unwrap();
            let parameters = I420Parameters::new(width, height, hdr, tone, matte)
                .unwrap()
                .words();
            cases.push(
                json!({"name":format!("{width}x{height}/{name}"),"width":width,"height":height,
                "hdr":hdr,"peak":peak,"matte":matte,"linear":image.pixels(),
                "parameters":parameters.as_slice(),"planes":result.planes()}),
            );
        }
    }
    println!(
        "{}",
        json!({"format":"musteroffice.gpu-i420-vectors/1",
        "shader":mo_video_gpu_plan::i420_output_shader(),"cases":cases})
    );
}
