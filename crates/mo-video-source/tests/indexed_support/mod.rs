use super::support;
use mo_media::DecodeTimeline;
use mo_media_isobmff::{ByteRange, Edit, Sample, TrackIndex};
use mo_video_compile::MediaBinding;
use mo_video_model::MediaStream;
use mo_video_source::indexed::*;
use std::{cell::Cell, rc::Rc};

pub fn binding() -> MediaBinding {
    let mut binding = support::binding();
    let MediaStream::Video { available, .. } = &mut binding.descriptor else {
        unreachable!()
    };
    *available = support::range(0, 200);
    binding
}
pub fn decoder() -> support::Fake {
    let mut decoder = support::Fake::new(vec![0, 30, 60, 90]);
    decoder.stream.container_track_id = Some(7);
    decoder.timeline = DecodeTimeline::RawIsoBmff;
    decoder
}
pub fn raw_track() -> TrackIndex {
    let entries = [
        (20, None, 65536),
        (60, Some(0), 98304),
        (40, Some(0), 65536),
        (40, Some(60), 0),
        (20, None, 65536),
        (20, Some(120), 65536),
    ];
    let mut start = 0;
    let edits = entries
        .into_iter()
        .map(|(duration, media, rate)| {
            let edit = Edit {
                movie_start: start,
                movie_duration: duration,
                media_time: media,
                rate_16_16: rate,
            };
            start += duration;
            edit
        })
        .collect();
    TrackIndex {
        id: 7,
        flags: 1,
        handler: *b"vide",
        declared_track_duration: Some(200),
        timescale: 1000,
        declared_media_duration: Some(120),
        decode_duration: 120,
        display_matrix: [0; 9],
        display_size_16_16: [65536; 2],
        descriptions: vec![],
        auxiliary: vec![],
        edits,
        samples: [0, 90, 30, 60]
            .into_iter()
            .enumerate()
            .map(|(i, pts)| Sample {
                bytes: ByteRange {
                    offset: i as u64,
                    length: 1,
                },
                dts: i as i64 * 30,
                pts,
                decode_duration: 30,
                description: 0,
                sync_sample: true,
            })
            .collect(),
    }
}
pub fn timing() -> VideoTiming {
    VideoTiming::build(&raw_track(), 1000, Default::default(), &|| false).unwrap()
}
pub struct Timeline {
    inner: VideoTiming,
    summary: VideoTimingSummary,
    pub calls: Rc<Cell<usize>>,
    pub drops: Rc<Cell<usize>>,
    pub fault: u8,
}
impl Timeline {
    pub fn new() -> Self {
        let inner = timing();
        let summary = inner.summary().clone();
        Self {
            inner,
            summary,
            calls: Rc::default(),
            drops: Rc::default(),
            fault: 0,
        }
    }
}
impl Drop for Timeline {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
impl VideoTimeline for Timeline {
    type Error = TimingError;
    fn summary(&self) -> &VideoTimingSummary {
        &self.summary
    }
    fn select(
        &mut self,
        time: &ExactRatio,
        side: Side,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VideoTimeSelection, TimingError> {
        self.calls.set(self.calls.get() + 1);
        match self.fault {
            1 => return Ok(VideoTimeSelection::Outside),
            2 => self.summary.track_id = 8,
            3 => panic!("timeline failure"),
            _ => {}
        }
        self.inner.select(time, side, cancelled)
    }
}
