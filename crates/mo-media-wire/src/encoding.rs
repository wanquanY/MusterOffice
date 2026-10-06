//! Allocation admission for packed encoder payloads. Native codec configuration
//! admission remains authoritative; this layer bounds bytes before any raw read.
use mo_media::{EncodeConfig, EncodeLimits};
use std::io;
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "encoder wire dimensions or byte budget",
    )
}
pub fn encode_video_lengths(c: EncodeConfig, l: EncodeLimits) -> io::Result<[usize; 3]> {
    if !(2..=8192).contains(&c.width)
        || !(2..=8192).contains(&c.height)
        || !c.width.is_multiple_of(2)
        || !c.height.is_multiple_of(2)
        || l.max_frame_bytes == 0
        || l.max_frame_bytes > 512 << 20
        || l.max_file_bytes == 0
        || l.max_file_bytes > i64::MAX as u64
        || l.max_write_bytes < l.max_file_bytes
    {
        return Err(invalid());
    }
    let pixels = u64::from(c.width) * u64::from(c.height);
    if pixels > u64::from(l.max_pixels) || pixels + pixels / 2 > l.max_frame_bytes {
        return Err(invalid());
    }
    let n = usize::try_from(pixels).map_err(|_| invalid())?;
    Ok([n, n / 4, n / 4])
}
pub fn encode_audio_bytes(c: EncodeConfig, l: EncodeLimits, frames: u32) -> io::Result<usize> {
    if !(1..=65536).contains(&frames)
        || !(1..=2).contains(&c.audio_channels)
        || c.audio_samples <= 0
    {
        return Err(invalid());
    }
    let n = u64::from(frames) * u64::from(c.audio_channels) * 4;
    if n > l.max_frame_bytes {
        return Err(invalid());
    }
    usize::try_from(n).map_err(|_| invalid())
}
pub fn read_encode_bytes(input: &mut impl io::Read, n: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(n).map_err(io::Error::other)?;
    bytes.resize(n, 0);
    input.read_exact(&mut bytes)?;
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> EncodeConfig {
        EncodeConfig {
            profile: mo_media::EncodeProfile::Ffv1PcmMp4,
            width: 128,
            height: 96,
            video_time_scale: 25,
            frame_rate_num: 25,
            frame_rate_den: 1,
            video_duration: 1,
            video_frames: 1,
            video_bit_rate: 1000000,
            audio_sample_rate: 48000,
            audio_channels: 2,
            audio_bit_rate: 128000,
            gop_frames: 1,
            audio_samples: 1920,
        }
    }
    #[test]
    fn packed_encoder_allocation_is_admitted_before_media_payload() {
        let c = config();
        let limits = EncodeLimits::default();
        assert_eq!(
            encode_video_lengths(c, limits).unwrap(),
            [12288, 3072, 3072]
        );
        for (w, h) in [(0, 96), (129, 96), (128, 95), (u32::MAX, u32::MAX)] {
            let mut bad = c;
            bad.width = w;
            bad.height = h;
            assert!(encode_video_lengths(bad, limits).is_err());
        }
        assert!(
            encode_video_lengths(
                c,
                EncodeLimits {
                    max_frame_bytes: 18431,
                    ..limits
                }
            )
            .is_err()
        );
        for frames in [0, 65537, u32::MAX] {
            assert!(encode_audio_bytes(c, limits, frames).is_err());
        }
        assert_eq!(encode_audio_bytes(c, limits, 65536).unwrap(), 524288);
        assert!(
            encode_audio_bytes(
                c,
                EncodeLimits {
                    max_frame_bytes: 524287,
                    ..limits
                },
                65536
            )
            .is_err()
        );
        let mut silent = c;
        silent.audio_samples = 0;
        assert!(encode_audio_bytes(silent, limits, 1).is_err());
        assert!(read_encode_bytes(&mut std::io::Cursor::new([0; 3]), 4).is_err());
    }
}
