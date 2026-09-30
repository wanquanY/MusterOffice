use mo_charts::{DecimalNumber, NumberError, sectors::*};
use mo_geometry::Fixed;
use std::cell::Cell;

const TURN: i128 = 1i128 << 32;
fn number(s: &str) -> DecimalNumber {
    s.to_owned().try_into().unwrap()
}
fn request(values: &[&str]) -> SectorRequest {
    SectorRequest {
        start_turn: Fixed::ZERO,
        direction: SectorDirection::Clockwise,
        negative_weights: NegativeWeights::Reject,
        weights: values
            .iter()
            .enumerate()
            .map(|(i, s)| SectorWeight {
                point_index: (i * 3 + 2) as u32,
                value: number(s),
            })
            .collect(),
    }
}
fn run(request: &SectorRequest) -> SectorLayout {
    layout(request, SectorLimits::default(), &|| false).unwrap()
}

#[test]
fn decimal_spelling_sign_and_precision_survive_json_roundtrip() {
    for text in [
        "1",
        " +001.2300e+002 \n",
        ".5",
        "1.",
        "-0.000e-50",
        "1.0000000000000001",
        "1e308",
        "1e-4096",
    ] {
        let value = number(text);
        assert_eq!(value.lexical(), text);
        let wire = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<DecimalNumber>(&wire).unwrap(), value);
    }
    assert!(number("-0.00").is_zero());
    assert!(number("-0.00").is_sign_negative());
    assert!(!number("0.00000000000000001").is_zero());
    assert!(serde_json::from_str::<DecimalNumber>("1.5").is_err());
}

#[test]
fn malformed_nonfinite_and_unbounded_numbers_are_rejected() {
    for text in [
        "",
        " ",
        ".",
        "+",
        "1e",
        "1e+",
        "1.2.3",
        "1e1e2",
        "NaN",
        "INF",
        "-Infinity",
        "#N/A",
        "1,000",
        "1 0",
        "１２",
        "\u{a0}1",
        "1\0",
    ] {
        assert_eq!(
            DecimalNumber::try_from(text.to_owned()),
            Err(NumberError::Invalid),
            "{text:?}"
        );
    }
    for text in [
        "1e4097".to_owned(),
        "1e-4097".into(),
        "1e9999999999999999999999".into(),
        "1".repeat(1025),
    ] {
        assert_eq!(DecimalNumber::try_from(text), Err(NumberError::Limit));
    }
}

#[test]
fn exact_prefix_rounding_closes_a_whole_turn_without_accumulated_drift() {
    let thirds = run(&request(&["1", "1", "1"]));
    assert_eq!(
        thirds
            .sectors
            .iter()
            .map(|s| s.end_turn.raw())
            .collect::<Vec<_>>(),
        [1431655765, 2863311531, TURN]
    );
    for pair in thirds.sectors.windows(2) {
        assert_eq!(pair[0].end_turn, pair[1].start_turn);
    }
    assert_eq!(
        thirds
            .sectors
            .iter()
            .map(|s| s.point_index)
            .collect::<Vec<_>>(),
        [2, 5, 8]
    );
    assert_eq!(thirds.endpoint_error_bound.raw(), 1);
    let decimal = run(&request(&["0.1", "0.2", "0.7"]));
    let integer = run(&request(&["1", "2", "7"]));
    assert_eq!(decimal.sectors, integer.sectors);
}

#[test]
fn finite_values_outside_binary_float_range_still_allocate_exactly() {
    let small = run(&request(&["1e-400", "2e-400", "7e-400"]));
    let large = run(&request(&["1e400", "2e400", "7e400"]));
    let baseline = run(&request(&["1", "2", "7"]));
    assert_eq!(small.sectors, baseline.sectors);
    assert_eq!(large.sectors, baseline.sectors);
    assert_eq!(
        run(&request(&["1e4096", "1e4096"])).sectors[0]
            .end_turn
            .raw(),
        TURN / 2
    );
}

#[test]
fn zero_and_subresolution_positive_points_are_never_silently_removed() {
    let result = run(&request(&["0", "1e-100", "1", "-0"]));
    assert_eq!(result.sectors.len(), 4);
    assert!(result.sectors[0].zero_weight);
    assert!(!result.sectors[0].positive_below_resolution);
    assert!(!result.sectors[1].zero_weight);
    assert!(result.sectors[1].positive_below_resolution);
    assert_eq!(result.sectors[2].end_turn.raw(), TURN);
    assert_eq!(result.sectors[3].start_turn.raw(), TURN);
    let all_zero = run(&request(&["0", "-0", "0e500"]));
    assert!(all_zero.zero_total);
    assert_eq!(all_zero.endpoint_error_bound, Fixed::ZERO);
    assert!(
        all_zero
            .sectors
            .iter()
            .all(|s| s.zero_weight && s.start_turn == s.end_turn)
    );
    assert!(run(&request(&[])).zero_total);
}

#[test]
fn direction_start_and_negative_policy_are_explicit() {
    let mut input = request(&["-1", "3"]);
    assert_eq!(
        layout(&input, SectorLimits::default(), &|| false),
        Err(SectorError::NegativeWeight(2))
    );
    input.negative_weights = NegativeWeights::AbsoluteMagnitude;
    input.start_turn = Fixed::from_raw(TURN / 4);
    input.direction = SectorDirection::Counterclockwise;
    let result = run(&input);
    assert_eq!(result.sectors[0].start_turn.raw(), TURN / 4);
    assert_eq!(result.sectors[0].end_turn.raw(), 0);
    assert_eq!(result.sectors[1].end_turn.raw(), -3 * TURN / 4);
    assert_eq!(input.weights[0].value.lexical(), "-1");
    input.start_turn = Fixed::from_raw(TURN);
    assert!(matches!(
        layout(&input, SectorLimits::default(), &|| false),
        Err(SectorError::Invalid(_))
    ));
}

#[test]
fn point_identity_and_all_resource_budgets_fail_without_a_partial_plan() {
    let mut input = request(&["1", "2"]);
    input.weights[1].point_index = input.weights[0].point_index;
    assert_eq!(
        layout(&input, SectorLimits::default(), &|| false),
        Err(SectorError::DuplicatePoint(2))
    );
    let defaults = SectorLimits::default();
    for (limits, expected) in [
        (
            SectorLimits {
                max_points: 1,
                ..defaults
            },
            "points",
        ),
        (
            SectorLimits {
                max_number_bytes: 1,
                ..defaults
            },
            "number bytes",
        ),
        (
            SectorLimits {
                max_scaled_decimal_digits: 1,
                ..defaults
            },
            "scaled decimal digits",
        ),
    ] {
        assert_eq!(
            layout(&request(&["1", "2"]), limits, &|| false),
            Err(SectorError::Limit(expected))
        );
    }
    assert_eq!(
        layout(
            &request(&["1", "2"]),
            SectorLimits {
                max_boundary_decimal_digits: 1,
                ..defaults
            },
            &|| false
        ),
        Err(SectorError::Limit("boundary decimal digits"))
    );
    assert!(matches!(
        layout(
            &request(&["1e4096", "1e-4096"]),
            SectorLimits {
                max_scaled_decimal_digits: 8192,
                ..defaults
            },
            &|| false
        ),
        Err(SectorError::Limit("scaled decimal digits"))
    ));
}

#[test]
fn cancellation_is_observed_in_preflight_expansion_and_boundary_allocation() {
    let input = request(&["1", "2", "3", "4"]);
    for stop in 0..=4 * 4 + 1 {
        let calls = Cell::new(0);
        let result = layout(&input, SectorLimits::default(), &|| {
            let n = calls.get();
            calls.set(n + 1);
            n == stop
        });
        assert_eq!(result, Err(SectorError::Cancelled), "cancel at {stop}");
    }
}

#[test]
fn exhaustive_small_integer_oracle_bounds_every_endpoint_and_preserves_order() {
    // Independent integer oracle: compare quantized endpoints against the exact
    // rational equation, including ties. No production decimal parsing involved.
    for a in 0u64..=8 {
        for b in 0u64..=8 {
            for c in 0u64..=8 {
                let total = a + b + c;
                let owned = [a.to_string(), b.to_string(), c.to_string()];
                let result = run(&request(&owned.each_ref().map(String::as_str)));
                let mut prefix = 0u64;
                for (index, v) in [a, b, c].into_iter().enumerate() {
                    prefix += v;
                    let edge = result.sectors[index].end_turn.raw();
                    if total > 0 {
                        let residual = edge * i128::from(total) - i128::from(prefix) * TURN;
                        assert!(residual.abs() * 2 <= i128::from(total));
                        if residual.abs() * 2 == i128::from(total) {
                            assert!(residual >= 0);
                        }
                    } else {
                        assert_eq!(edge, 0);
                    }
                    assert!(edge >= result.sectors[index].start_turn.raw());
                }
                if total > 0 {
                    assert_eq!(result.sectors.last().unwrap().end_turn.raw(), TURN);
                }
            }
        }
    }
}
