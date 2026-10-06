#![allow(dead_code)]
use mo_color::rgb::{Encoding, Primaries, Transfer};
use mo_common::{Digest, RationalTime};
use mo_media::*;
use mo_video_compile::*;
use mo_video_model::*;
use mo_video_render::*;
pub fn info(format: &str, w: u32, h: u32) -> FrameInfo {
    FrameInfo {
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
        format: format.into(),
        channel_layout: String::new(),
        colors: CodecColorTags {
            primaries: 1,
            transfer: 1,
            matrix: 1,
            range: 1,
            chroma_location: 1,
        },
        sample_aspect_ratio: Some(TimeBase {
            numerator: 1,
            denominator: 1,
        }),
        alpha_mode: 2,
        key_frame: true,
        interlaced: false,
        top_field_first: false,
    }
}
pub fn plane(row: usize, rows: usize, bytes: Vec<u8>) -> FramePlane {
    FramePlane {
        row_bytes: row,
        rows,
        bytes,
    }
}
pub fn rgba(pixels: &[[u8; 4]]) -> DecodedFrame {
    DecodedFrame {
        info: info("rgba", pixels.len() as u32, 1),
        planes: vec![plane(
            pixels.len() * 4,
            1,
            pixels.iter().flatten().copied().collect(),
        )],
    }
}
pub fn srgb() -> InputColor {
    InputColor {
        encoding: Encoding {
            primaries: Primaries::Bt709,
            transfer: Transfer::Srgb,
        },
        matrix: Matrix::Rgb,
        range: SignalRange::Full,
        chroma: None,
        alpha: Alpha::Straight,
    }
}
pub fn output() -> OutputOptions {
    OutputOptions {
        tone_map: None,
        alpha: OutputAlpha::Straight,
        max_bytes: 1 << 20,
    }
}
pub fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
pub fn node(id: &str, source: VisualSource) -> VisualNode {
    VisualNode {
        track: TrackId::new("v1").unwrap(),
        clip: ClipId::new(id).unwrap(),
        local: TimeSample::after(t(0)),
        effects: vec![],
        source,
    }
}
pub fn plan(layers: Vec<VisualNode>, w: u32, h: u32) -> VideoFramePlan {
    VideoFramePlan {
        document_digest: Digest::from_sha256([1; 32]),
        frame_index: mo_common::Ticks::new(0),
        sample: TimeSample::after(t(0)),
        canvas: Canvas {
            width: w,
            height: h,
        },
        layers,
    }
}
pub struct NoMedia;
impl VisualResources for NoMedia {
    fn media(
        &mut self,
        _: &MediaBinding,
        _: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> mo_video_render::Result<ResolvedMedia> {
        panic!("no media");
    }
}
