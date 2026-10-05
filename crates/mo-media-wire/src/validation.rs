use crate::{MAX_PLANES, PlaneLayout};
use mo_media::{DecodeLimits, FrameInfo, MediaKind, StreamInfo, TimeBase};
use std::io;
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn validate_limits(limits: DecodeLimits, byte_length: u64) -> io::Result<()> {
    if byte_length == 0
        || byte_length > i64::MAX as u64
        || byte_length > limits.max_input_bytes
        || limits.max_read_bytes == 0
        || limits.max_frame_bytes == 0
        || limits.max_frame_bytes > 512 << 20
        || limits.max_streams == 0
        || limits.max_streams > 1024
        || limits.max_pixels == 0
        || limits.max_audio_samples == 0
        || limits.max_channels == 0
        || limits.max_channels > 64
        || limits.max_decode_steps == 0
    {
        return Err(invalid("media input or admission limits"));
    }
    Ok(())
}
pub fn plane_lengths(planes: &[PlaneLayout], max_bytes: u64) -> io::Result<Vec<usize>> {
    if planes.is_empty() || planes.len() > MAX_PLANES {
        return Err(invalid("plane count"));
    }
    let mut total = 0u64;
    planes
        .iter()
        .map(|plane| {
            let bytes = plane
                .rows
                .checked_mul(plane.row_bytes)
                .filter(|n| *n > 0)
                .ok_or_else(|| invalid("plane dimensions overflow or empty"))?;
            total = total
                .checked_add(bytes)
                .filter(|n| *n <= max_bytes)
                .ok_or_else(|| invalid("aggregate plane budget"))?;
            usize::try_from(bytes).map_err(|_| invalid("addressable plane bytes"))
        })
        .collect()
}
fn ratio(value: TimeBase) -> bool {
    value.numerator > 0 && value.denominator > 0
}
fn optional_positive(value: Option<u32>) -> bool {
    value.is_none_or(|v| v > 0)
}
pub fn validate_streams(streams: &[StreamInfo], limits: DecodeLimits) -> io::Result<()> {
    if streams.len() > limits.max_streams as usize {
        return Err(invalid("stream count budget"));
    }
    for (index, s) in streams.iter().enumerate() {
        if s.index != index as u32
            || s.codec.len() > 63
            || s.channel_layout.len() > 127
            || !s.time_base.is_none_or(ratio)
            || !s.nominal_frame_rate.is_none_or(ratio)
            || !s.sample_aspect_ratio.is_none_or(ratio)
            || !optional_positive(s.width)
            || !optional_positive(s.height)
            || !optional_positive(s.sample_rate)
            || !s.channels.is_none_or(|n| n > 0 && n <= limits.max_channels)
            || u64::from(s.width.unwrap_or(0)) * u64::from(s.height.unwrap_or(0))
                > u64::from(limits.max_pixels)
        {
            return Err(invalid("invalid stream metadata"));
        }
    }
    Ok(())
}
pub fn validate_frame(
    info: &FrameInfo,
    planes: &[PlaneLayout],
    limits: DecodeLimits,
) -> io::Result<()> {
    plane_lengths(planes, limits.max_frame_bytes)?;
    if info.generation == 0
        || !ratio(info.time_base)
        || !info.sample_aspect_ratio.is_none_or(ratio)
        || info.format.is_empty()
        || info.format.len() > 63
        || info.channel_layout.len() > 127
        || info.duration_ticks.is_some_and(|n| n <= 0)
    {
        return Err(invalid("invalid frame metadata"));
    }
    match info.kind {
        MediaKind::Video => {
            let (Some(w), Some(h)) = (info.width, info.height) else {
                return Err(invalid("missing dimensions"));
            };
            if w == 0
                || h == 0
                || u64::from(w) * u64::from(h) > u64::from(limits.max_pixels)
                || planes.len() > 4
            {
                return Err(invalid("video dimensions or plane count"));
            }
        }
        MediaKind::Audio => {
            if info.sample_rate.is_none_or(|n| n == 0)
                || !info
                    .channels
                    .is_some_and(|n| n > 0 && n <= limits.max_channels)
                || !info
                    .samples
                    .is_some_and(|n| n > 0 && n <= limits.max_audio_samples)
                || planes.iter().any(|p| p.rows != 1)
                || !(planes.len() == 1 || Some(planes.len() as u32) == info.channels)
            {
                return Err(invalid("audio samples or channel layout"));
            }
        }
        MediaKind::Other => return Err(invalid("frame is not audio/video")),
    }
    Ok(())
}
