//! Finite output inspection shared by native and WASM hosts. This verifies the
//! exact container/sample clocks against a frozen export configuration; it is
//! NOT a decoder or a publication receipt. Hosts must additionally hash sealed
//! bytes, decode the returned spot requests, and atomically publish their result.
use mo_media::{EncodeConfig, EncodeProfile};
use mo_media_isobmff::{IsoBmffIndex, TrackIndex};
use mo_time::ExactRatio;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum InspectionError {
    #[error("encoded output differs from the export configuration")]
    Mismatch,
    #[error("output inspection cancelled")]
    Cancelled,
    #[error("unsupported output inspection profile")]
    Unsupported,
}
type Result<T> = std::result::Result<T, InspectionError>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportInspection {
    pub profile: &'static str,
    pub video_track: u32,
    pub audio_track: Option<u32>,
    pub video_frames: u32,
    pub audio_samples: String,
    pub movie_timescale: u32,
    pub movie_duration: String,
    /// Exact presentation positions at which a host must prove actual decode.
    pub video_spots: Vec<ExactRatio>,
}
fn demand(value: bool) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(InspectionError::Mismatch)
    }
}
fn active(cancelled: &dyn Fn() -> bool) -> Result<()> {
    if cancelled() {
        Err(InspectionError::Cancelled)
    } else {
        Ok(())
    }
}
fn scaled(ticks: u64, from: u32, to: u32) -> Result<u64> {
    demand(from > 0 && to > 0)?;
    let value = u128::from(ticks) * u128::from(to);
    demand(value % u128::from(from) == 0)?;
    u64::try_from(value / u128::from(from)).map_err(|_| InspectionError::Mismatch)
}
fn ratio(ticks: u64, scale: u32) -> Result<ExactRatio> {
    ExactRatio::new(ticks.into(), scale.into(), 256).map_err(|_| InspectionError::Mismatch)
}
/// Only the explicit browser ordered-AVC/AAC profile is admitted here. Native
/// reordered AVC remains a different profile, never silently accepted as I/P.
pub fn inspect_browser_candidate(
    index: &IsoBmffIndex,
    config: EncodeConfig,
    cancelled: &dyn Fn() -> bool,
) -> Result<ExportInspection> {
    active(cancelled)?;
    if config.profile != EncodeProfile::H264WebCodecsAacMp4 {
        return Err(InspectionError::Unsupported);
    }
    demand(
        config.width > 0
            && config.width <= 8192
            && config.height > 0
            && config.height <= 8192
            && config.video_frames > 0
            && config.video_frames <= 2_000_000
            && config.video_duration > 0
            && config.video_time_scale > 0
            && config.frame_rate_num > 0
            && config.frame_rate_den > 0
            && config.audio_sample_rate > 0
            && config.audio_samples >= 0
            && matches!(config.audio_channels, 0..=2)
            && (config.audio_samples > 0) == (config.audio_channels > 0),
    )?;
    demand(index.tracks.len() == if config.audio_samples > 0 { 2 } else { 1 })?;
    let videos: Vec<_> = index
        .tracks
        .iter()
        .filter(|t| t.handler == *b"vide")
        .collect();
    let audios: Vec<_> = index
        .tracks
        .iter()
        .filter(|t| t.handler == *b"soun")
        .collect();
    demand(videos.len() == 1 && audios.len() == usize::from(config.audio_samples > 0))?;
    let video = videos[0];
    let duration = config.video_duration as u64;
    let movie_video = scaled(duration, config.video_time_scale, index.movie_timescale)?;
    let movie_audio = scaled(
        config.audio_samples as u64,
        config.audio_sample_rate,
        index.movie_timescale,
    )?;
    demand(index.declared_movie_duration == Some(movie_video.max(movie_audio)))?;
    demand(
        video.timescale == config.video_time_scale
            && video.samples.len() == config.video_frames as usize
            && video.declared_media_duration == Some(duration)
            && video.decode_duration == duration
            && video.display_size_16_16 == [config.width << 16, config.height << 16]
            && video.display_matrix == [65536, 0, 0, 0, 65536, 0, 0, 0, 1073741824]
            && video.descriptions.len() == 1
            && video.descriptions[0].format == *b"avc1",
    )?;
    presentation(video, movie_video, duration, Some(0))?;
    let full = scaled(
        u64::from(config.frame_rate_den),
        config.frame_rate_num,
        config.video_time_scale,
    )?;
    demand(
        full > 0
            && u128::from(duration) > u128::from(full) * u128::from(config.video_frames - 1)
            && u128::from(duration) <= u128::from(full) * u128::from(config.video_frames),
    )?;
    for (ordinal, sample) in video.samples.iter().enumerate() {
        active(cancelled)?;
        let start = full
            .checked_mul(ordinal as u64)
            .ok_or(InspectionError::Mismatch)?;
        let span = (duration - start).min(full);
        demand(
            sample.pts >= 0
                && sample.pts as u64 == start
                && sample.dts == sample.pts
                && u64::from(sample.decode_duration) == span
                && sample.description == 0
                && sample.bytes.length > 0
                && (ordinal != 0 || sample.sync_sample),
        )?;
    }
    if let Some(audio) = audios.first() {
        demand(
            audio.id != video.id
                && audio.timescale == config.audio_sample_rate
                && audio.descriptions.len() == 1
                && audio.descriptions[0].format == *b"mp4a"
                && !audio.samples.is_empty(),
        )?;
        let origin = presentation(audio, movie_audio, config.audio_samples as u64, None)?;
        let end = origin
            .checked_add(config.audio_samples as u64)
            .ok_or(InspectionError::Mismatch)?;
        let mut at = 0u64;
        for sample in &audio.samples {
            active(cancelled)?;
            demand(
                sample.pts >= 0
                    && sample.pts as u64 == at
                    && sample.dts == sample.pts
                    && sample.decode_duration > 0
                    && sample.decode_duration <= 1024
                    && sample.description == 0
                    && sample.bytes.length > 0,
            )?;
            at = at
                .checked_add(u64::from(sample.decode_duration))
                .ok_or(InspectionError::Mismatch)?;
        }
        demand(
            at >= end
                && at - end < 1024
                && origin < at
                && audio.decode_duration == at
                && audio.declared_media_duration == Some(at),
        )?;
    }
    let mut ordinals = vec![0, config.video_frames / 2, config.video_frames - 1];
    ordinals.sort_unstable();
    ordinals.dedup();
    let video_spots = ordinals
        .into_iter()
        .map(|i| ratio(u64::from(i) * full, config.video_time_scale))
        .collect::<Result<_>>()?;
    active(cancelled)?;
    Ok(ExportInspection {
        profile: "musteroffice.browser-export-inspection/1-draft",
        video_track: video.id,
        audio_track: audios.first().map(|a| a.id),
        video_frames: config.video_frames,
        audio_samples: config.audio_samples.to_string(),
        movie_timescale: index.movie_timescale,
        movie_duration: movie_video.max(movie_audio).to_string(),
        video_spots,
    })
}
/// A single unit-rate presentation segment; codec priming remains an explicit
/// raw-media origin, with no browser-specific delay constants or duration repair.
fn presentation(
    track: &TrackIndex,
    movie_duration: u64,
    media_duration: u64,
    origin: Option<u64>,
) -> Result<u64> {
    demand(
        track.id > 0
            && track.declared_track_duration == Some(movie_duration)
            && track.edits.len() <= 1,
    )?;
    let actual = if let Some(edit) = track.edits.first() {
        demand(
            edit.movie_start == 0
                && edit.movie_duration == movie_duration
                && edit.rate_16_16 == 65536,
        )?;
        u64::try_from(edit.media_time.ok_or(InspectionError::Mismatch)?)
            .map_err(|_| InspectionError::Mismatch)?
    } else {
        0
    };
    demand(
        origin.is_none_or(|expected| actual == expected)
            && track.decode_duration
                >= actual
                    .checked_add(media_duration)
                    .ok_or(InspectionError::Mismatch)?,
    )?;
    Ok(actual)
}
