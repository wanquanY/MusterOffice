#![allow(dead_code)]
use mo_common::{Digest, RationalTime, ResourceId};
use mo_media::*;
use mo_video_compile::MediaBinding;
use mo_video_model::*;
use mo_video_source::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub fn time(n: i64) -> RationalTime {
    RationalTime::new(n, 1000).unwrap()
}
pub fn range(a: i64, b: i64) -> TimeRange {
    TimeRange {
        start: time(a),
        end: time(b),
    }
}
pub fn sample(t: i64, side: SampleSide) -> TimeSample {
    TimeSample {
        time: mo_time::ExactRatio::from_time(time(t)),
        side,
    }
}
pub fn binding() -> MediaBinding {
    MediaBinding {
        resource: ResourceId::new("movie").unwrap(),
        digest: Digest::from_sha256([1; 32]),
        stream: StreamId::new("video0").unwrap(),
        descriptor: MediaStream::Video {
            available: range(-50, 160),
            dimensions: Canvas {
                width: 1,
                height: 1,
            },
            frame_rate: None,
        },
    }
}
pub fn colors() -> CodecColorTags {
    CodecColorTags {
        primaries: 1,
        transfer: 1,
        matrix: 1,
        range: 1,
        chroma_location: 0,
    }
}
pub fn stream() -> StreamInfo {
    StreamInfo {
        index: 0,
        kind: MediaKind::Video,
        codec: "rawvideo".into(),
        time_base: Some(TimeBase {
            numerator: 1,
            denominator: 1000,
        }),
        start_ticks: Some(-50),
        duration_ticks: Some(210),
        width: Some(1),
        height: Some(1),
        sample_rate: None,
        channels: None,
        nominal_frame_rate: None,
        sample_aspect_ratio: None,
        colors: colors(),
        channel_layout: String::new(),
        display_matrix: None,
    }
}
pub fn frame(pts: i64, generation: u64) -> DecodedFrame {
    DecodedFrame {
        info: FrameInfo {
            generation,
            kind: MediaKind::Video,
            time_base: TimeBase {
                numerator: 1,
                denominator: 1000,
            },
            pts_ticks: Some(pts),
            duration_ticks: Some(1000),
            width: Some(1),
            height: Some(1),
            sample_rate: None,
            channels: None,
            samples: None,
            format: "gray".into(),
            channel_layout: String::new(),
            colors: colors(),
            sample_aspect_ratio: None,
            alpha_mode: 0,
            key_frame: true,
            interlaced: false,
            top_field_first: false,
        },
        planes: vec![FramePlane {
            row_bytes: 1,
            rows: 1,
            bytes: vec![pts as u8],
        }],
    }
}
#[derive(Default)]
pub struct Observed {
    pub reads: Cell<usize>,
    pub seeks: RefCell<Vec<i64>>,
    pub drops: Cell<usize>,
}
pub struct Fake {
    pub pts: Vec<i64>,
    pub cursor: usize,
    pub generation: u64,
    pub stream: StreamInfo,
    pub observed: Rc<Observed>,
    pub overshoot: bool,
    pub corrupt: Option<&'static str>,
}
impl Fake {
    pub fn new(pts: Vec<i64>) -> Self {
        Self {
            pts,
            cursor: 0,
            generation: 1,
            stream: stream(),
            observed: Rc::default(),
            overshoot: false,
            corrupt: None,
        }
    }
}
impl Drop for Fake {
    fn drop(&mut self) {
        self.observed.drops.set(self.observed.drops.get() + 1);
    }
}
impl VideoDecoder for Fake {
    type Error = std::io::Error;
    fn stream(&self) -> &StreamInfo {
        &self.stream
    }
    fn generation(&self) -> u64 {
        self.generation
    }
    fn next(&mut self, _: &dyn Fn() -> bool) -> Result<Option<DecodedFrame>, Self::Error> {
        self.observed.reads.set(self.observed.reads.get() + 1);
        let Some(pts) = self.pts.get(self.cursor) else {
            return Ok(None);
        };
        self.cursor += 1;
        let mut out = frame(*pts, self.generation);
        match self.corrupt {
            Some("pts") => out.info.pts_ticks = None,
            Some("generation") => out.info.generation = 0,
            Some("layout") => out.planes[0].rows = 2,
            Some("oversize") => {
                out.planes[0].bytes.resize(2, 0);
                out.planes[0].row_bytes = 2;
            }
            Some("dimensions") => out.info.width = Some(2),
            _ => (),
        }
        Ok(Some(out))
    }
    fn seek(&mut self, ticks: i64, _: &dyn Fn() -> bool) -> Result<(), Self::Error> {
        self.observed.seeks.borrow_mut().push(ticks);
        self.generation += 1;
        self.cursor = if ticks <= self.stream.start_ticks.unwrap() {
            0
        } else if self.overshoot {
            self.pts.partition_point(|p| *p <= ticks)
        } else {
            self.pts.partition_point(|p| *p <= ticks).saturating_sub(1) / 2 * 2
        };
        Ok(())
    }
}
pub fn limits() -> SourceLimits {
    SourceLimits {
        max_frame_bytes: 1,
        max_retained_bytes: 5,
        max_retained_frames: 5,
        max_decode_calls_per_query: 100,
        exact_bits: 256,
    }
}
pub fn selected(result: SampledVideo) -> PresentedFrame {
    let SourceSelection::Frame(frame) = result.selection else {
        panic!("{:?}", result.selection);
    };
    frame
}
