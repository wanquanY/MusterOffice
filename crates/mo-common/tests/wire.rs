use mo_common::*;
use proptest::prelude::*;
use serde_json::json;

proptest! {
    #[test]
    fn every_u64_byte_length_round_trips(value in any::<u64>()) {
        let length = ByteLength::new(value);
        let wire = serde_json::to_string(&length).unwrap();
        prop_assert_eq!(serde_json::from_str::<ByteLength>(&wire).unwrap(), length);
        prop_assert_eq!(canonical_bytes(&length).unwrap(), wire.as_bytes());
    }
    #[test]
    fn every_i64_coordinate_round_trips(value in any::<i64>()) {
        let emu = Emu::new(value);
        let wire = serde_json::to_string(&emu).unwrap();
        prop_assert_eq!(&wire, &format!("\"{value}\""));
        prop_assert_eq!(serde_json::from_str::<Emu>(&wire).unwrap(), emu);
    }
    #[test]
    fn rational_normalization_preserves_time(ticks in any::<i64>(), scale in 1_u32..=1_000_000_000) {
        let time = RationalTime::new(ticks, scale).unwrap();
        prop_assert_eq!(time.compare_time(time.normalized()), std::cmp::Ordering::Equal);
        prop_assert_eq!(time.normalized().normalized(), time.normalized());
    }
}

#[test]
fn byte_lengths_reject_numeric_loss_overflow_and_noncanonical_spellings() {
    for bad in [
        "1",
        "\"-1\"",
        "\"01\"",
        "\"+1\"",
        "\"1\\n\"",
        "\"18446744073709551616\"",
    ] {
        assert!(
            serde_json::from_str::<ByteLength>(bad).is_err(),
            "accepted {bad}"
        );
    }
    let value = ByteLength::new(u64::MAX);
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        "\"18446744073709551615\""
    );
}

#[test]
fn coordinates_reject_lossy_or_noncanonical_inputs() {
    for bad in [
        "0",
        "1.5",
        "\"-0\"",
        "\"01\"",
        "\"+1\"",
        "\"1e3\"",
        "\" 1\"",
        "\"9223372036854775808\"",
        "\"-9223372036854775809\"",
    ] {
        assert!(serde_json::from_str::<Emu>(bad).is_err(), "accepted {bad}");
    }
    assert!(Emu::new(i64::MAX).checked_add(Emu::new(1)).is_err());
    assert!(Emu::new(i64::MIN).checked_sub(Emu::new(1)).is_err());
}

#[test]
fn identifiers_are_constrained_on_deserialization() {
    for bad in ["", "../secret", "a b", "a/b", "é", "_start"] {
        assert!(DocumentId::new(bad).is_err());
        assert!(serde_json::from_value::<DocumentId>(json!(bad)).is_err());
    }
    assert!(SlideId::new("slide:1_a-b.c").is_ok());
    assert!(ObjectId::new("a".repeat(129)).is_err());
}

#[test]
fn time_is_exact_at_extremes_and_ntsc_rate() {
    let a = RationalTime::new(30_000, 1_001).unwrap();
    assert!(a.compare_time(RationalTime::new(30, 1).unwrap()).is_lt());
    let min = RationalTime::new(i64::MIN, 1_000_000_000).unwrap();
    let max = RationalTime::new(i64::MAX, 1_000_000_000).unwrap();
    assert!(min.compare_time(max).is_lt());
    assert_eq!(
        RationalTime::new(0, 3).unwrap().normalized(),
        RationalTime::new(0, 1).unwrap()
    );
    for scale in [0, 1_000_000_001, u32::MAX] {
        assert!(RationalTime::new(0, scale).is_err());
    }
}

#[test]
fn canonical_bytes_have_stable_keys_and_preserve_text() {
    assert_eq!(
        canonical_bytes(&json!({"z":1,"a":{"β":true,"A":"e\u{301}"}})).unwrap(),
        "{\"a\":{\"A\":\"e\u{301}\",\"β\":true},\"z\":1}".as_bytes()
    );
    assert_ne!(
        digest("test", &"é").unwrap(),
        digest("test", &"e\u{301}").unwrap()
    );
    assert_ne!(digest("a", &1).unwrap(), digest("b", &1).unwrap());
    for value in [json!(1.0), json!(9_007_199_254_740_992_u64)] {
        assert!(canonical_bytes(&value).is_err());
    }
}

#[test]
fn duplicate_keys_are_rejected_at_every_depth() {
    for input in [r#"{"id":1,"id":2}"#, r#"{"nested":[{"a":1,"\u0061":2}]}"#] {
        assert!(from_json_str::<serde_json::Value>(input).is_err());
    }
    assert_eq!(
        from_json_str::<serde_json::Value>(r#"{"a":[null,true,-1,1.5,"é"]}"#).unwrap(),
        json!({"a":[null,true,-1,1.5,"é"]})
    );
}

#[test]
fn non_finite_numbers_cannot_silently_become_null_in_a_digest() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0] {
        assert!(canonical_bytes(&Some(vec![value])).is_err());
        assert!(digest("test", &Some(vec![value])).is_err());
    }
    assert!(canonical_bytes(&Option::<u32>::None).is_ok());
}

#[test]
fn structural_request_errors_preserve_field_path_and_reject_duplicates() {
    #[derive(serde::Deserialize)]
    struct Input {
        pages: Vec<Page>,
    }
    #[derive(serde::Deserialize)]
    struct Page {
        width: u32,
    }
    let error = mo_common::from_json_str_with_path::<Input>(r#"{"pages":[{"width":"wide"}]}"#)
        .err()
        .unwrap();
    assert_eq!(error.path, "pages[0].width");
    assert!(error.message.contains("u32"));
    assert!(mo_common::from_json_str_with_path::<Input>(r#"{"pages":[],"pages":[]}"#).is_err());
    let value =
        mo_common::from_json_str_with_path::<Input>(r#"{"pages":[{"width":1280}]}"#).unwrap();
    assert_eq!(value.pages[0].width, 1280);
}
