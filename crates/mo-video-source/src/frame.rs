use crate::SourceLimits;
pub(crate) fn valid_limits(limits: SourceLimits, minimum_frames: usize) -> bool {
    (128..=4096).contains(&limits.exact_bits)
        && limits.max_frame_bytes > 0
        && limits.max_frame_bytes <= 512 << 20
        && limits.max_retained_bytes <= 1 << 30
        && limits.max_retained_frames >= minimum_frames
        && limits.max_retained_frames <= 100_000
        && limits.max_decode_calls_per_query > 0
        && limits.max_decode_calls_per_query <= 1_000_000
        && limits
            .max_frame_bytes
            .checked_mul(minimum_frames)
            .is_some_and(|n| n <= limits.max_retained_bytes)
}
pub(crate) fn payload_bytes(frame: &mo_media::DecodedFrame) -> Option<usize> {
    if frame.planes.is_empty() || frame.planes.len() > 4 {
        return None;
    }
    frame.planes.iter().try_fold(0usize, |sum, p| {
        let n = p.row_bytes.checked_mul(p.rows)?;
        if n == 0 || n != p.bytes.len() {
            return None;
        }
        sum.checked_add(n)
    })
}
