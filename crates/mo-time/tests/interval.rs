use mo_time::{ExactInterval, ExactRatio, SampleSide::*, TimeError};
#[test]
fn interval_keeps_unrepresentable_boundaries_and_one_sided_limits() {
    let boundary = ExactRatio::new(1.into(), 9_999_999_967u64.into(), 128).unwrap();
    assert_eq!(boundary.to_time_exact(), Err(TimeError::Representation));
    let range = ExactInterval {
        start: ExactRatio::integer(0),
        end: boundary.clone(),
    };
    range.validate(128).unwrap();
    assert!(range.contains(&range.start, After));
    assert!(!range.contains(&range.start, Before));
    assert!(range.contains(&boundary, Before));
    assert!(!range.contains(&boundary, After));
    assert_eq!(range.validate(16), Err(TimeError::Budget));
    let value = serde_json::to_value(&range).unwrap();
    let decoded: ExactInterval = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, range);
    for side in [After, Before] {
        assert_eq!(side.reversed().reversed(), side);
    }
    assert_eq!(serde_json::to_string(&Before).unwrap(), "\"before\"");
}
#[test]
fn interval_does_not_accept_empty_reversed_or_representation_equal_endpoints() {
    for end in [-1, 0] {
        let range = ExactInterval {
            start: ExactRatio::integer(0),
            end: ExactRatio::integer(end),
        };
        assert_eq!(range.validate(128), Err(TimeError::Interval));
    }
    let range = ExactInterval {
        start: ExactRatio::from_time(mo_common::RationalTime::new(2, 4).unwrap()),
        end: ExactRatio::new(1.into(), 2.into(), 128).unwrap(),
    };
    assert_eq!(range.validate(128), Err(TimeError::Interval));
}
