use super::*;
fn reserve<T>(out: &mut Vec<T>, n: usize) -> Result<()> {
    out.try_reserve_exact(n).map_err(|_| TimingError::Budget)
}
pub(super) fn build(
    track: &TrackIndex,
    movie_scale: u32,
    limits: TimingLimits,
    check: &dyn Fn() -> bool,
) -> Result<VideoTiming> {
    active(check)?;
    if movie_scale == 0 || track.timescale == 0 || track.id == 0 {
        return Err(TimingError::Invalid("track clock"));
    }
    if track.handler != *b"vide" {
        return Err(TimingError::Unsupported("video timing required"));
    }
    if limits.max_samples == 0
        || limits.max_samples > 10_000_000
        || limits.max_edits > 100_000
        || !(32..=4096).contains(&limits.exact_bits)
        || track.samples.len() > limits.max_samples
        || track.edits.len() > limits.max_edits
    {
        return Err(TimingError::Budget);
    }
    let bits = limits.exact_bits;
    let mut samples = Vec::new();
    reserve(&mut samples, track.samples.len())?;
    let mut dts = 0i64;
    for (index, sample) in track.samples.iter().enumerate() {
        active(check)?;
        if sample.dts != dts || sample.decode_duration == 0 {
            return Err(TimingError::Invalid("decode sample clock"));
        }
        dts = dts
            .checked_add(i64::from(sample.decode_duration))
            .ok_or(TimingError::Invalid("decode clock overflow"))?;
        samples.push(Raw {
            pts: sample.pts,
            dts: sample.dts,
            duration: sample.decode_duration,
            index: index as u32,
            access: 0,
        });
    }
    if dts as u64 != track.decode_duration {
        return Err(TimingError::Invalid("decode duration mismatch"));
    }
    sort(&mut samples, check)?;
    for pair in samples.windows(2) {
        active(check)?;
        if pair[0].pts == pair[1].pts {
            return Err(TimingError::Unsupported(
                "coincident video composition times",
            ));
        }
    }
    let span = if let (Some(first), Some(last)) = (samples.first(), samples.last()) {
        Some((
            first.pts,
            last.pts
                .checked_add(i64::from(last.duration))
                .ok_or(TimingError::Invalid("composition end overflow"))?,
        ))
    } else {
        None
    };
    let access = super::access::build(track, &mut samples, check)?;
    let clock = |ticks: u64| -> Result<ExactRatio> {
        Ok(ExactRatio::new(ticks.into(), movie_scale.into(), bits)?)
    };
    let mut segments = Vec::new();
    reserve(&mut segments, track.edits.len().max(1))?;
    let end = if track.edits.is_empty() {
        let end = match track.declared_track_duration {
            Some(t) => clock(t)?,
            None => seconds(span.map_or(0, |(_, end)| end.max(0)), track.timescale, bits)?,
        };
        if end.positive() {
            segments.push(Segment {
                movie: Interval {
                    start: ExactRatio::integer(0),
                    end: end.clone(),
                },
                media: Some(ExactRatio::integer(0)),
                rate: ExactRatio::integer(1),
                index: None,
            });
        }
        end
    } else {
        let mut end = 0u64;
        for (i, edit) in track.edits.iter().enumerate() {
            active(check)?;
            if edit.movie_start != end || edit.media_time.is_some_and(|t| t < 0) {
                return Err(TimingError::Invalid("edit clock sequence"));
            }
            if edit.rate_16_16 < 0 {
                return Err(TimingError::Unsupported("negative container edit rate"));
            }
            end = end
                .checked_add(edit.movie_duration)
                .ok_or(TimingError::Invalid("edit end overflow"))?;
            if edit.movie_duration == 0 {
                continue;
            }
            segments.push(Segment {
                movie: Interval {
                    start: clock(edit.movie_start)?,
                    end: clock(end)?,
                },
                media: edit
                    .media_time
                    .map(|t| seconds(t, track.timescale, bits))
                    .transpose()?,
                rate: seconds(i64::from(edit.rate_16_16), 65536, bits)?,
                index: Some(i as u32),
            });
        }
        clock(end)?
    };
    active(check)?;
    Ok(VideoTiming {
        summary: VideoTimingSummary {
            track_id: track.id,
            movie_timescale: movie_scale,
            media_timescale: track.timescale,
            samples: samples.len() as u32,
            edits: track.edits.len() as u32,
            raw_span: span,
            movie_end: end,
        },
        samples,
        access,
        segments,
        bits,
    })
}
/// Iterative merge sort: O(n log n), one bounded scratch allocation, with
/// cancellation between merges and every 1024 elements of a large merge.
fn sort(samples: &mut Vec<Raw>, check: &dyn Fn() -> bool) -> Result<()> {
    let n = samples.len();
    if n < 2 {
        return Ok(());
    }
    let mut scratch = Vec::new();
    reserve(&mut scratch, n)?;
    scratch.extend_from_slice(samples);
    let mut width = 1;
    while width < n {
        for start in (0..n).step_by(width * 2) {
            active(check)?;
            let middle = (start + width).min(n);
            let end = (middle + width).min(n);
            let (mut left, mut right) = (start, middle);
            for (offset, out) in scratch[start..end].iter_mut().enumerate() {
                if offset % 1024 == 0 {
                    active(check)?;
                }
                if left < middle && (right == end || samples[left].pts <= samples[right].pts) {
                    *out = samples[left];
                    left += 1;
                } else {
                    *out = samples[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(samples, &mut scratch);
        width *= 2;
    }
    Ok(())
}
