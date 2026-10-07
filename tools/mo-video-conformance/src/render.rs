//! Test-only CPU pixel vectors across native/WASM. Not a published renderer ABI.
use mo_common::from_json_str_with_path;
use mo_media::{DecodedFrame, FrameInfo, FramePlane};
use mo_video_compile::{MediaBinding, VideoFramePlan};
use mo_video_model::TimeSample;
use mo_video_render::*;
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Plane {
    row_bytes: usize,
    rows: usize,
    bytes: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Frame {
    info: FrameInfo,
    planes: Vec<Plane>,
    has_alpha: bool,
    hlg_display: bool,
    tone_map: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    frames: Vec<Frame>,
    plans: Vec<VideoFramePlan>,
    #[serde(default)]
    viewports: Vec<ViewportFixture>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewportFixture {
    plan: VideoFramePlan,
    width: u32,
    height: u32,
}
struct NoResources;
impl VisualResources for NoResources {
    fn media(
        &mut self,
        _: &MediaBinding,
        _: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        Err(RenderError::Resource(
            "fixture has no external media".into(),
        ))
    }
}
fn record(image: LinearImage, tone_map: bool) -> Result<Value> {
    let options = OutputOptions {
        tone_map: if tone_map {
            Some(ToneMap::ReinhardLuminanceV1 {
                source_peak_nits: 1000.,
            })
        } else {
            None
        },
        alpha: OutputAlpha::Straight,
        max_bytes: 1 << 20,
    };
    let pixels = image
        .pixels()
        .iter()
        .map(|p| p.map(f32::to_bits))
        .collect::<Vec<_>>();
    let output = image.to_srgb8(options, &|| false)?;
    Ok(
        json!({"linearBits":pixels,"rgba":output.rgba,"gamutClipped":output.gamut_clipped_pixels,"toneMapped":output.tone_mapped_pixels}),
    )
}
fn outcome(result: Result<Value>) -> Value {
    match result {
        Ok(value) => json!({"status":"rendered","value":value}),
        Err(e) => json!({"status":"rejected","error":e.to_string()}),
    }
}
fn run(input: &str) -> std::result::Result<Value, String> {
    if input.len() > 1 << 20 {
        return Err("render fixture byte limit".into());
    }
    let fixture: Fixture = from_json_str_with_path(input).map_err(|e| e.to_string())?;
    if fixture.frames.len() + fixture.plans.len() + fixture.viewports.len() > 128 {
        return Err("render fixture count limit".into());
    }
    let mut results = vec![];
    for frame in fixture.frames {
        let normalized = (|| {
            let color = InputColor::from_codec(
                &frame.info,
                frame.has_alpha,
                if frame.hlg_display {
                    Some(HlgDisplay {
                        peak_nits: 1000.,
                        system_gamma: 1.2,
                    })
                } else {
                    None
                },
            )?;
            let decoded = DecodedFrame {
                info: frame.info,
                planes: frame
                    .planes
                    .into_iter()
                    .map(|p| FramePlane {
                        row_bytes: p.row_bytes,
                        rows: p.rows,
                        bytes: p.bytes,
                    })
                    .collect(),
            };
            let result = normalize_frame(
                &decoded,
                color,
                PixelLimits {
                    max_pixels: 4096,
                    max_bytes: 4096 * 16,
                },
                &|| false,
            )?;
            let mut value = record(result.image, frame.tone_map)?;
            value["signalClipped"] = json!(result.clipped_signal_pixels);
            Ok(value)
        })();
        results.push(outcome(normalized));
    }
    for plan in fixture.plans {
        let result = render_frame(
            &plan,
            &mut NoResources,
            Sampling::Bilinear,
            RenderLimits {
                pixels: PixelLimits {
                    max_pixels: 4096,
                    max_bytes: 4096 * 16,
                },
                max_nodes: 128,
                max_effects: 128,
                max_pixel_visits: 1 << 20,
                max_temporary_bytes: 1 << 20,
                ..Default::default()
            },
            &|| false,
        );
        results.push(outcome(result.and_then(|r| record(r.image, false))));
    }
    for viewport in fixture.viewports {
        let result = render_frame_with_options(
            &viewport.plan,
            &mut NoResources,
            FrameRenderOptions {
                raster: RasterExtent {
                    width: viewport.width,
                    height: viewport.height,
                },
                sampling: Sampling::Bilinear,
                limits: RenderLimits {
                    pixels: PixelLimits {
                        max_pixels: 4096,
                        max_bytes: 4096 * 16,
                    },
                    max_nodes: 128,
                    max_effects: 128,
                    max_pixel_visits: 1 << 20,
                    max_temporary_bytes: 1 << 20,
                    ..Default::default()
                },
            },
            &|| false,
        );
        results.push(outcome(result.and_then(|r| record(r.image, false))));
    }
    Ok(json!({"status":"complete","results":results}))
}
#[wasm_bindgen]
pub fn video_render_conformance(input: &str) -> String {
    match run(input) {
        Ok(v) => v.to_string(),
        Err(e) => json!({"status":"invalidFixture","error":e}).to_string(),
    }
}
