//! Validate worker results against the retained, immutable track summary.
use super::{
    ExactRatio, IndexLimits, IndexSummary, MediaTimeSide, VideoTimeSelection, VideoTimingSummary,
};
#[cfg(test)]
use mo_media_isobmff::presentation::Interval;

pub(super) fn valid_summary(
    value: &VideoTimingSummary,
    index: IndexSummary,
    limits: IndexLimits,
) -> bool {
    value.validate(256).is_ok()
        && value.movie_timescale == index.movie_timescale
        && value.samples as usize <= limits.max_samples
        && value.edits as usize <= limits.max_edits
}

pub(super) fn valid_selection(
    value: &VideoTimeSelection,
    summary: &VideoTimingSummary,
    time: &ExactRatio,
    side: MediaTimeSide,
) -> bool {
    value.validate(summary, time, side, 256).is_ok()
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
                decode_start: 10,
                access: Some(mo_media_isobmff::presentation::VideoAccess {
                    sample: 0,
                    decode_start: 0,
                }),
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
                ..
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
