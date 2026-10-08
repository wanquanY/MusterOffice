//! CPU oracle spans packed/semi-planar/high-bit-depth, alpha and display EOTFs.
use mo_media::{CodecColorTags, DecodedFrame, FrameInfo, FramePlane, MediaKind, TimeBase};
use mo_video_gpu_plan::{BrowserColorSpace, BrowserFrameInput, BrowserHlgDisplay};
use mo_video_render::{HlgDisplay, InputColor, PixelLimits, normalize_frame};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for (format, native, sx, sy, semi, alpha, bits) in [
        ("I420", "yuv420p", 2u32, 2u32, false, false, 8u32),
        ("I422", "yuv422p", 2, 1, false, false, 8),
        ("I444", "yuv444p", 1, 1, false, false, 8),
        ("NV12", "nv12", 2, 2, true, false, 8),
        ("I420A", "yuva420p", 2, 2, false, true, 8),
        ("I444A", "yuva444p", 1, 1, false, true, 8),
        ("I420P10", "yuv420p10le", 2, 2, false, false, 10),
        ("I422P10", "yuv422p10le", 2, 1, false, false, 10),
        ("I444P10", "yuv444p10le", 1, 1, false, false, 10),
        ("I420P12", "yuv420p12le", 2, 2, false, false, 12),
        ("I444AP10", "yuva444p10le", 1, 1, false, true, 10),
    ] {
        for (transfer, code, primaries, matrix, full_range, chroma) in [
            ("bt709", 1, "bt709", 1, false, "left"),
            ("iec61966-2-1", 13, "smpte170m", 6, true, "center"),
            ("pq", 16, "bt2020", 9, false, "topLeft"),
            ("hlg", 18, "bt2020", 9, true, "bottom"),
        ] {
            let (w, h) = (6u32, 4u32);
            let bpc = if bits > 8 { 2 } else { 1 };
            let max = (1 << bits) - 1;
            let mut shapes = vec![
                (w, h, 1),
                (w.div_ceil(sx), h.div_ceil(sy), if semi { 2 } else { 1 }),
            ];
            if !semi {
                shapes.push((w.div_ceil(sx), h.div_ceil(sy), 1));
            }
            if alpha {
                shapes.push((w, h, 1));
            }
            let planes: Vec<_> = shapes
                .iter()
                .enumerate()
                .map(|(i, (pw, ph, c))| {
                    let mut bytes = Vec::new();
                    for k in 0..pw * ph * c {
                        let v = ((k * 73 + i as u32 * 127) * (max / 255)) & max;
                        if bpc == 1 {
                            bytes.push(v as u8)
                        } else {
                            bytes.extend_from_slice(&(v as u16).to_le_bytes())
                        }
                    }
                    FramePlane {
                        row_bytes: (pw * c * bpc) as usize,
                        rows: *ph as usize,
                        bytes,
                    }
                })
                .collect();
            let c = CodecColorTags {
                primaries: if primaries == "bt709" {
                    1
                } else if primaries == "bt2020" {
                    9
                } else {
                    6
                },
                transfer: code,
                matrix,
                range: if full_range { 2 } else { 1 },
                chroma_location: match chroma {
                    "left" => 1,
                    "center" => 2,
                    "topLeft" => 3,
                    _ => 6,
                },
            };
            let f = DecodedFrame {
                info: FrameInfo {
                    generation: 1,
                    kind: MediaKind::Video,
                    time_base: TimeBase {
                        numerator: 1,
                        denominator: 30,
                    },
                    pts_ticks: Some(0),
                    duration_ticks: Some(1),
                    width: Some(w),
                    height: Some(h),
                    sample_rate: None,
                    channels: None,
                    samples: None,
                    format: native.into(),
                    channel_layout: String::new(),
                    colors: c,
                    sample_aspect_ratio: None,
                    alpha_mode: 2,
                    key_frame: true,
                    interlaced: false,
                    top_field_first: false,
                },
                planes,
            };
            let hdr = Some(HlgDisplay {
                peak_nits: 1000.,
                system_gamma: 1.2,
            });
            let linear = normalize_frame(
                &f,
                InputColor::from_frame(&f.info, hdr)?,
                PixelLimits::default(),
                &|| false,
            )?
            .image;
            let source = BrowserFrameInput {
                width: w,
                height: h,
                format: format.into(),
                color_space: BrowserColorSpace {
                    primaries: primaries.into(),
                    transfer: transfer.into(),
                    matrix: match matrix {
                        1 => "bt709",
                        6 => "smpte170m",
                        _ => "bt2020-ncl",
                    }
                    .into(),
                    full_range,
                },
                chroma_location: Some(chroma.into()),
                hlg_display: Some(BrowserHlgDisplay {
                    peak_nits: 1000.,
                    system_gamma: 1.2,
                }),
            };
            let plan = source.compile()?;
            let mut bytes = vec![0u8; plan.byte_length as usize];
            for (plane, copy) in f.planes.iter().zip(&plan.planes) {
                for y in 0..plane.rows {
                    let out = copy.offset as usize + y * copy.stride as usize;
                    bytes[out..out + plane.row_bytes].copy_from_slice(
                        &plane.bytes[y * plane.row_bytes..(y + 1) * plane.row_bytes],
                    );
                }
            }
            cases.push(json!({"name":format!("{format}/{transfer}"),"plan":plan,"bytes":bytes,"linear":linear.pixels()}));
        }
    }
    println!("{}", json!(cases));
    Ok(())
}
