//! Independent CPU color oracle for browser GPU boundary checks.
use mo_color::rgb::{Encoding, Primaries, RgbTransform, Transfer};
use mo_video_gpu_plan::{ColorParameters, RAW_INPUT_SHADER, srgb_output_shader};
use mo_video_render::{LinearImage, OutputAlpha, OutputOptions, PixelLimits, ToneMap};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rgba: Vec<[u8; 4]> = (0..24)
        .map(|i| {
            [
                (i * 31 % 256) as u8,
                (i * 47 % 256) as u8,
                (i * 61 % 256) as u8,
                [0, 1, 64, 128, 192, 255][i % 6],
            ]
        })
        .collect();
    let transform = RgbTransform::new(
        Encoding {
            primaries: Primaries::Bt709,
            transfer: Transfer::Srgb,
        },
        Primaries::Bt2020,
    )?;
    let linear = rgba
        .iter()
        .map(|p| {
            let rgb = transform
                .decode([p[0], p[1], p[2]].map(|v| f64::from(v) / 255.))
                .unwrap()
                .rgb;
            let a = f64::from(p[3]) / 255.;
            [
                (rgb[0] * a) as f32,
                (rgb[1] * a) as f32,
                (rgb[2] * a) as f32,
                a as f32,
            ]
        })
        .collect::<Vec<_>>();
    let image =
        LinearImage::from_pixels(6, 4, linear.clone(), false, PixelLimits::default(), &|| {
            false
        })?;
    let straight = image
        .to_srgb8(
            OutputOptions {
                tone_map: None,
                alpha: OutputAlpha::Straight,
                max_bytes: 1024,
            },
            &|| false,
        )?
        .rgba;
    let premul = image
        .to_srgb8(
            OutputOptions {
                tone_map: None,
                alpha: OutputAlpha::PremultipliedEncoded,
                max_bytes: 1024,
            },
            &|| false,
        )?
        .rgba;
    let hdr = LinearImage::from_pixels(
        2,
        2,
        vec![
            [5., 2., -0.1, 1.],
            [2., -0.2, 1., 0.5],
            [0.; 4],
            [3., 4., 5., 1.],
        ],
        true,
        PixelLimits::default(),
        &|| false,
    )?;
    let tone = Some(ToneMap::ReinhardLuminanceV1 {
        source_peak_nits: 1000.,
    });
    let hdr8 = hdr
        .to_srgb8(
            OutputOptions {
                tone_map: tone,
                alpha: OutputAlpha::Straight,
                max_bytes: 1024,
            },
            &|| false,
        )?
        .rgba;
    println!(
        "{}",
        json!({"rgba":rgba,"linear":linear,"straight":straight,"premultiplied":premul,
        
        "straightParameters":ColorParameters::srgb_output(6,4,false,None,OutputAlpha::Straight)?.words(),
        "premultipliedParameters":ColorParameters::srgb_output(6,4,false,None,OutputAlpha::PremultipliedEncoded)?.words(),
        "hdr":{"linear":hdr.pixels(),"rgba":hdr8,"parameters":ColorParameters::srgb_output(2,2,true,tone,OutputAlpha::Straight)?.words()},
        "shaders":{"profile":"musteroffice.video.gpu-color/1-draft","input":RAW_INPUT_SHADER,"output":srgb_output_shader()}})
    );
    Ok(())
}
