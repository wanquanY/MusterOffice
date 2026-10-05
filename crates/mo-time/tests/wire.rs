use mo_common::RationalTime;
use mo_time::{ExactRatio, TimeError};
use serde_json::json;

#[test]
fn exact_wire_preserves_values_beyond_javascript_and_document_time_limits() {
    let ratio = ExactRatio::new(
        9_007_199_254_740_993i64.into(),
        1_000_000_001i64.into(),
        256,
    )
    .unwrap();
    let wire = serde_json::to_value(&ratio).unwrap();
    assert_eq!(
        wire,
        json!({"numerator":"9007199254740993","denominator":"1000000001"})
    );
    assert_eq!(serde_json::from_value::<ExactRatio>(wire).unwrap(), ratio);
    assert_eq!(ratio.to_time_exact(), Err(TimeError::Representation));
    let original = ExactRatio::from_time(RationalTime::new(6, 4).unwrap());
    assert_eq!(
        serde_json::to_value(&original).unwrap(),
        json!({"numerator":"3","denominator":"2"})
    );
    // Serialization must not change the representation retained for existing PPT contracts.
    assert_eq!(original.numerator().to_string(), "6");
}

#[test]
fn exact_wire_rejects_ambiguous_decimals_unknown_fields_and_excessive_values() {
    for text in ["+1", "-0", "00", "01", " 1", "1 ", "1\n", "1e3", "1.0", ""] {
        assert!(
            serde_json::from_value::<ExactRatio>(json!({"numerator":text,"denominator":"1"}))
                .is_err(),
            "{text:?}"
        );
    }
    for denominator in ["0", "-1", "01", "+1"] {
        assert!(
            serde_json::from_value::<ExactRatio>(
                json!({"numerator":"1","denominator":denominator})
            )
            .is_err()
        );
    }
    assert!(
        serde_json::from_value::<ExactRatio>(json!({"numerator":"1","denominator":"1","extra":0}))
            .is_err()
    );
    assert!(
        serde_json::from_str::<ExactRatio>(
            r#"{"numerator":"1","numerator":"2","denominator":"1"}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ExactRatio>(
            json!({"numerator":"9".repeat(1235),"denominator":"1"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ExactRatio>(
            json!({"numerator":"1","denominator":"9".repeat(1236)})
        )
        .is_err()
    );
}

#[test]
fn computed_wire_admission_does_not_override_smaller_runtime_profiles() {
    let value =
        ExactRatio::new((num_bigint::BigInt::from(1) << 4095u32) - 1, 1.into(), 4096).unwrap();
    let decoded: ExactRatio =
        serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(decoded.check_budget(256), Err(TimeError::Budget));
}
