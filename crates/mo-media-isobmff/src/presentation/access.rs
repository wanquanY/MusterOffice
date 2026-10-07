//! Offline two-order predecessor queries. A B picture can follow an open-GOP
//! intra picture in decode order but precede it in presentation order; that
//! later-presented intra picture is not a sufficient random-access candidate.
use super::*;

pub(super) fn build(
    track: &TrackIndex,
    sorted: &mut [Raw],
    check: &dyn Fn() -> bool,
) -> Result<Vec<VideoAccess>> {
    let mut prefix = Vec::new();
    prefix
        .try_reserve_exact(sorted.len() + 1)
        .map_err(|_| TimingError::Budget)?;
    prefix.resize(sorted.len() + 1, 0u32);
    let mut access = Vec::new();
    let mut sync_count = 0;
    for (i, sample) in track.samples.iter().enumerate() {
        if i % 1024 == 0 {
            active(check)?;
        }
        sync_count += usize::from(sample.sync_sample);
    }
    access
        .try_reserve_exact(sync_count)
        .map_err(|_| TimingError::Budget)?;
    for (decode_index, sample) in track.samples.iter().enumerate() {
        active(check)?;
        let position = sorted
            .binary_search_by_key(&sample.pts, |s| s.pts)
            .map_err(|_| TimingError::Invalid("access sample absent"))?;
        if sample.sync_sample {
            access.push(VideoAccess {
                sample: decode_index as u32,
                decode_start: sample.dts,
            });
            let candidate = access.len() as u32;
            let mut at = position + 1;
            while at < prefix.len() {
                prefix[at] = prefix[at].max(candidate);
                at += at & at.wrapping_neg();
            }
        }
        let mut at = position + 1;
        let mut candidate = 0;
        while at > 0 {
            candidate = candidate.max(prefix[at]);
            at -= at & at.wrapping_neg();
        }
        sorted[position].access = candidate;
    }
    active(check)?;
    Ok(access)
}
