//! One exact finite schedule for synchronous/native and asynchronous/browser
//! drivers. Codec transport quantization never changes authoring or mux time.
use mo_common::Ticks;
use mo_media::EncodeConfig;
use mo_time::{ExactRatio, Rounding};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("sample position is outside the immutable export plan")]
    Range,
    #[error("sample acknowledgement does not match the next export step")]
    Acknowledgement,
    #[error("exact interval cannot be represented by the codec transport clock")]
    Transport,
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExportProgress {
    pub video_frames: u32,
    pub audio_samples: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportVideoSample {
    pub index: u32,
    pub pts: Ticks,
    pub duration: Ticks,
    pub timescale: u32,
    pub key_frame: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportAudioSample {
    pub first_sample: Ticks,
    pub frames: u32,
    pub sample_rate: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ExportWork {
    Video(ExportVideoSample),
    Audio(ExportAudioSample),
    Complete,
}
/// WebCodecs uses integer microseconds. Both absolute endpoints are quantized
/// once, ties to even. Residuals retain the difference from exact media time;
/// packets must be associated back to their exact plan intervals for muxing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCodecInterval {
    pub timestamp_us: Ticks,
    pub duration_us: Ticks,
    pub start_residual: ExactRatio,
    pub end_residual: ExactRatio,
}
fn codec_interval(
    start: i64,
    length: i64,
    scale: u32,
) -> Result<ExportCodecInterval, ScheduleError> {
    if start < 0 || length <= 0 || scale == 0 {
        return Err(ScheduleError::Transport);
    }
    let end = start.checked_add(length).ok_or(ScheduleError::Transport)?;
    let quantize = |value| {
        ExactRatio::integer(value)
            .div(&ExactRatio::integer(i64::from(scale)), 256)?
            .quantize(1_000_000, Rounding::NearestTiesEven, 256)
    };
    let a = quantize(start)?;
    let b = quantize(end)?;
    // JS API timestamps must survive an exact conversion to Number. No clamping
    // or artificial 1us tail may silently extend a sub-microsecond final frame.
    if b.value.ticks.get() > 9_007_199_254_740_991 || b.value.ticks <= a.value.ticks {
        return Err(ScheduleError::Transport);
    }
    Ok(ExportCodecInterval {
        timestamp_us: a.value.ticks,
        duration_us: Ticks::new(b.value.ticks.get() - a.value.ticks.get()),
        start_residual: a.residual,
        end_residual: b.residual,
    })
}
impl ExportVideoSample {
    pub fn codec_interval(&self) -> Result<ExportCodecInterval, ScheduleError> {
        codec_interval(self.pts.get(), self.duration.get(), self.timescale)
    }
}
impl ExportAudioSample {
    pub fn codec_interval(&self) -> Result<ExportCodecInterval, ScheduleError> {
        codec_interval(
            self.first_sample.get(),
            i64::from(self.frames),
            self.sample_rate,
        )
    }
}
/// Constructed only from a validated ExportPlan. No file, codec, wall clock or
/// task authority. A pending source/codec operation leaves next() unchanged.
#[derive(Debug, Clone)]
pub struct ExportSchedule {
    config: EncodeConfig,
    period: i64,
    audio_block_frames: u32,
    progress: ExportProgress,
}
impl ExportSchedule {
    pub(crate) fn new(config: EncodeConfig, period: i64, audio_block_frames: u32) -> Self {
        Self {
            config,
            period,
            audio_block_frames,
            progress: ExportProgress::default(),
        }
    }
    pub fn progress(&self) -> ExportProgress {
        self.progress
    }
    pub fn video(&self, index: u32) -> Result<ExportVideoSample, ScheduleError> {
        if index >= self.config.video_frames {
            return Err(ScheduleError::Range);
        }
        let pts = i64::from(index)
            .checked_mul(self.period)
            .ok_or(ScheduleError::Range)?;
        Ok(ExportVideoSample {
            index,
            pts: Ticks::new(pts),
            duration: Ticks::new(self.period.min(self.config.video_duration - pts)),
            timescale: self.config.video_time_scale,
            key_frame: index.is_multiple_of(self.config.gop_frames),
        })
    }
    pub fn next(&self) -> ExportWork {
        let c = self.config;
        let v = self.progress.video_frames;
        let a = self.progress.audio_samples;
        if v == c.video_frames && a == c.audio_samples {
            return ExportWork::Complete;
        }
        // Absolute integer cross-products, identical for every driver. Audio
        // block partition cannot change frame positions or the finite tail.
        let video = v < c.video_frames
            && (a == c.audio_samples
                || u128::from(v) * self.period as u128 * u128::from(c.audio_sample_rate)
                    <= a as u128 * u128::from(c.video_time_scale));
        if video {
            ExportWork::Video(self.video(v).expect("validated finite export schedule"))
        } else {
            ExportWork::Audio(ExportAudioSample {
                first_sample: Ticks::new(a),
                frames: (c.audio_samples - a).min(i64::from(self.audio_block_frames)) as u32,
                sample_rate: c.audio_sample_rate,
            })
        }
    }
    /// Call only after the encoder has accepted this exact work. A rejected,
    /// duplicate or reordered acknowledgement cannot advance the cursor.
    pub fn accept(&mut self, work: ExportWork) -> Result<ExportProgress, ScheduleError> {
        if work == ExportWork::Complete || work != self.next() {
            return Err(ScheduleError::Acknowledgement);
        }
        match work {
            ExportWork::Video(_) => self.progress.video_frames += 1,
            ExportWork::Audio(a) => self.progress.audio_samples += i64::from(a.frames),
            ExportWork::Complete => unreachable!(),
        }
        Ok(self.progress)
    }
}
