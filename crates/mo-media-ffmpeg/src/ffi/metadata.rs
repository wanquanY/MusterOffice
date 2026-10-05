use super::bindings::{Frame, Stream};
use crate::{CodecColorTags, ErrorKind, FrameInfo, MediaError, MediaKind, StreamInfo, TimeBase};
use std::ffi::c_char;

pub(super) fn string(bytes: &[c_char]) -> String {
    let bytes: Vec<_> = bytes
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| *b as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}
fn positive(value: i32) -> Option<u32> {
    (value > 0).then_some(value as u32)
}
fn ratio(numerator: i32, denominator: i32) -> Option<TimeBase> {
    Some(TimeBase {
        numerator: positive(numerator)?,
        denominator: positive(denominator)?,
    })
}
fn kind(value: i32) -> Result<MediaKind, MediaError> {
    match value {
        1 => Ok(MediaKind::Video),
        2 => Ok(MediaKind::Audio),
        3 => Ok(MediaKind::Other),
        _ => Err(MediaError::new(
            ErrorKind::Abi,
            "unknown wrapper media kind",
        )),
    }
}
pub(super) fn stream(value: Stream) -> Result<StreamInfo, MediaError> {
    let index = u32::try_from(value.index)
        .map_err(|_| MediaError::new(ErrorKind::Abi, "negative stream index"))?;
    Ok(StreamInfo {
        index,
        kind: kind(value.kind)?,
        codec: string(&value.codec),
        time_base: ratio(value.time_base_num, value.time_base_den),
        start_ticks: (value.has_start != 0).then_some(value.start),
        duration_ticks: (value.has_duration != 0).then_some(value.duration),
        width: positive(value.width),
        height: positive(value.height),
        sample_rate: positive(value.sample_rate),
        channels: positive(value.channels),
        nominal_frame_rate: ratio(value.frame_rate_num, value.frame_rate_den),
        sample_aspect_ratio: ratio(value.sar_num, value.sar_den),
        colors: CodecColorTags {
            primaries: value.color_primaries,
            transfer: value.color_transfer,
            matrix: value.color_matrix,
            range: value.color_range,
            chroma_location: value.chroma_location,
        },
        channel_layout: string(&value.channel_layout),
        display_matrix: (value.has_display_matrix != 0).then_some(value.display_matrix),
    })
}
pub(super) fn frame(value: Frame) -> Result<FrameInfo, MediaError> {
    let time_base = ratio(value.time_base_num, value.time_base_den)
        .ok_or_else(|| MediaError::new(ErrorKind::Decode, "decoded frame has no valid timebase"))?;
    Ok(FrameInfo {
        generation: value.generation,
        kind: kind(value.kind)?,
        time_base,
        pts_ticks: (value.has_pts != 0).then_some(value.pts),
        duration_ticks: (value.has_duration != 0).then_some(value.duration),
        width: positive(value.width),
        height: positive(value.height),
        sample_rate: positive(value.sample_rate),
        channels: positive(value.channels),
        samples: positive(value.samples),
        format: string(&value.format),
        channel_layout: string(&value.channel_layout),
        colors: CodecColorTags {
            primaries: value.color_primaries,
            transfer: value.color_transfer,
            matrix: value.color_matrix,
            range: value.color_range,
            chroma_location: value.chroma_location,
        },
        sample_aspect_ratio: ratio(value.sar_num, value.sar_den),
        alpha_mode: value.alpha_mode,
        key_frame: value.key_frame != 0,
        interlaced: value.interlaced != 0,
        top_field_first: value.top_field_first != 0,
    })
}
