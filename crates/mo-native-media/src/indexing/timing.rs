//! Validate worker results against the retained, immutable track summary.
use super::{
    ExactRatio, IndexLimits, IndexSummary, MediaTimeSide, VideoTimeSelection, VideoTimingSummary,
};
use mo_media_isobmff::presentation::Interval;

pub(super) fn valid_summary(
    value: &VideoTimingSummary,
    index: IndexSummary,
    limits: IndexLimits,
) -> bool {
    value.track_id > 0
        && value.movie_timescale == index.movie_timescale
        && value.media_timescale > 0
        && value.samples as usize <= limits.max_samples
        && value.edits as usize <= limits.max_edits
        && value.movie_end.check_budget(256).is_ok()
        && value.movie_end.compare(&ExactRatio::integer(0)).is_ge()
        && match value.raw_span {
            Some((start, end)) => start < end && value.samples > 0,
            None => value.samples == 0,
        }
}

pub(super) fn valid_selection(
    value: &VideoTimeSelection,
    summary: &VideoTimingSummary,
    time: &ExactRatio,
    side: MediaTimeSide,
) -> bool {
    let full = Interval {
        start: ExactRatio::integer(0),
        end: summary.movie_end.clone(),
    };
    let valid_range = |range: &Interval, edit: Option<u32>| {
        range.validate(256).is_ok()
            && range.start.compare(&full.start).is_ge()
            && range.end.compare(&full.end).is_le()
            && range.contains(time, side)
            && match edit {
                None => summary.edits == 0,
                Some(index) => index < summary.edits,
            }
    };
    match value {
        VideoTimeSelection::Outside => !full.contains(time, side),
        VideoTimeSelection::Gap { movie, edit } => valid_range(movie, *edit),
        VideoTimeSelection::Sample {
            sample,
            edit,
            movie,
            raw_start,
            raw_end,
            media_time,
            media_side,
        } => {
            if *sample >= summary.samples
                || !valid_range(movie, *edit)
                || raw_start >= raw_end
                || !summary
                    .raw_span
                    .is_some_and(|(first, last)| first <= *raw_start && *raw_end <= last)
                || media_time.check_budget(256).is_err()
            {
                return false;
            }
            let raw = Interval {
                start: ExactRatio::integer(*raw_start),
                end: ExactRatio::integer(*raw_end),
            };
            media_time
                .mul(
                    &ExactRatio::integer(i64::from(summary.media_timescale)),
                    256,
                )
                .is_ok_and(|ticks| raw.contains(&ticks, *media_side))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (VideoTimingSummary, VideoTimeSelection) {
        (
            VideoTimingSummary {
                track_id: 9,
                movie_timescale: 1000,
                media_timescale: 10,
                samples: 3,
                edits: 2,
                raw_span: Some((0, 30)),
                movie_end: ExactRatio::integer(5),
            },
            VideoTimeSelection::Sample {
                sample: 1,
                edit: Some(1),
                movie: Interval {
                    start: ExactRatio::integer(1),
                    end: ExactRatio::integer(2),
                },
                raw_start: 10,
                raw_end: 20,
                media_time: ExactRatio::integer(1),
                media_side: MediaTimeSide::After,
            },
        )
    }
    #[test]
    fn bounded_result_must_belong_to_the_track_and_selected_side() {
        let (summary, good) = fixture();
        let time = ExactRatio::integer(1);
        let valid = |selection: &VideoTimeSelection| {
            valid_selection(selection, &summary, &time, MediaTimeSide::After)
        };
        assert!(valid(&good));
        assert!(!valid(&VideoTimeSelection::Outside));
        for field in 0..8 {
            let mut bad = good.clone();
            let VideoTimeSelection::Sample {
                sample,
                edit,
                movie,
                raw_start,
                raw_end,
                media_time,
                media_side,
            } = &mut bad
            else {
                unreachable!()
            };
            match field {
                0 => *sample = 3,
                1 => *edit = None,
                2 => *edit = Some(2),
                3 => movie.end = ExactRatio::integer(6),
                4 => *raw_start = -1,
                5 => *raw_end = 31,
                6 => *media_time = ExactRatio::integer(2),
                7 => *media_side = MediaTimeSide::Before,
                _ => unreachable!(),
            }
            assert!(!valid(&bad), "field {field}");
        }
        assert!(!valid_selection(
            &good,
            &summary,
            &time,
            MediaTimeSide::Before
        ));
        assert!(valid_selection(
            &VideoTimeSelection::Outside,
            &summary,
            &ExactRatio::integer(5),
            MediaTimeSide::After
        ));
        assert!(!valid_selection(
            &VideoTimeSelection::Outside,
            &summary,
            &ExactRatio::integer(5),
            MediaTimeSide::Before
        ));
    }
    #[test]
    fn no_edit_and_empty_tracks_have_explicit_boundaries() {
        let (mut summary, _) = fixture();
        summary.edits = 0;
        summary.samples = 0;
        summary.raw_span = None;
        let gap = VideoTimeSelection::Gap {
            movie: Interval {
                start: ExactRatio::integer(0),
                end: summary.movie_end.clone(),
            },
            edit: None,
        };
        assert!(valid_selection(
            &gap,
            &summary,
            &ExactRatio::integer(1),
            MediaTimeSide::After
        ));
        summary.movie_end = ExactRatio::integer(0);
        for side in [MediaTimeSide::Before, MediaTimeSide::After] {
            assert!(valid_selection(
                &VideoTimeSelection::Outside,
                &summary,
                &ExactRatio::integer(0),
                side
            ));
            assert!(!valid_selection(
                &gap,
                &summary,
                &ExactRatio::integer(1),
                side
            ));
        }
    }
}
