use mo_common::RationalTime;
use mo_time::{ExactRatio as R, TimeError};
use mo_video_model::{SampleSide as S, TimeAnchor, TimeMap, TimeRange, TimeSample};

fn time(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
fn sample(n: i64, side: S) -> TimeSample {
    TimeSample {
        time: R::integer(n),
        side,
    }
}
fn map(points: &[(i64, i64)]) -> TimeMap {
    TimeMap {
        anchors: points
            .iter()
            .map(|&(local, source)| TimeAnchor {
                local: time(local),
                source: time(source),
            })
            .collect(),
    }
}

#[test]
fn one_sided_limits_select_exact_adjacent_intervals_without_epsilon() {
    let a = TimeRange {
        start: time(0),
        end: time(2),
    };
    let b = TimeRange {
        start: time(2),
        end: time(5),
    };
    assert!(!sample(2, S::After).covers(a));
    assert!(sample(2, S::After).covers(b));
    assert!(sample(2, S::Before).covers(a));
    assert!(!sample(2, S::Before).covers(b));
    assert!(!sample(0, S::Before).covers(a));
    assert!(!sample(5, S::After).covers(b));
}

#[test]
fn reverse_flips_the_limit_and_double_reverse_restores_it() {
    let reverse = map(&[(0, 4), (4, 0)]);
    let mapped = reverse.sample(&sample(0, S::After), 256).unwrap().unwrap();
    assert_eq!(mapped.sample, sample(4, S::Before));
    assert_eq!(mapped.rate, R::integer(-1));
    let again = reverse.sample(&mapped.sample, 256).unwrap().unwrap();
    assert_eq!(again.sample, sample(0, S::After));
    assert!(
        reverse
            .sample(&sample(0, S::Before), 256)
            .unwrap()
            .is_none()
    );
    assert!(reverse.sample(&sample(4, S::After), 256).unwrap().is_none());
}

#[test]
fn slope_breaks_use_the_selected_segment_and_hold_uses_the_fixed_source_sample() {
    let curve = map(&[(0, 0), (2, 2), (4, 0), (6, 0)]);
    let left = curve.sample(&sample(2, S::Before), 256).unwrap().unwrap();
    let right = curve.sample(&sample(2, S::After), 256).unwrap().unwrap();
    assert_eq!(left.sample, sample(2, S::Before));
    assert_eq!(left.rate, R::integer(1));
    assert_eq!(right.sample, sample(2, S::Before));
    assert_eq!(right.rate, R::integer(-1));
    let hold = curve.sample(&sample(6, S::Before), 256).unwrap().unwrap();
    assert_eq!(hold.sample, sample(0, S::After));
    assert_eq!(hold.rate, R::integer(0));
}

#[test]
fn nesting_keeps_exact_intermediate_times_that_document_wire_cannot_store() {
    let local = TimeSample {
        time: R::new(1.into(), 1_000_000_001i64.into(), 256).unwrap(),
        side: S::After,
    };
    let doubled = map(&[(0, 0), (1, 2)]).sample(&local, 256).unwrap().unwrap();
    assert_eq!(
        doubled.sample.time.to_time_exact(),
        Err(TimeError::Representation)
    );
    let original = map(&[(0, 0), (2, 1)])
        .sample(&doubled.sample, 256)
        .unwrap()
        .unwrap();
    assert_eq!(original.sample, local);
    assert_eq!(
        map(&[(0, 0), (1, 2)]).sample(&local, 1),
        Err(TimeError::Budget)
    );
}
