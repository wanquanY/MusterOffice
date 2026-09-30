use mo_charts::{DecimalNumber, sectors::*};
fn weights(values: &[&str]) -> Vec<SectorWeight> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| SectorWeight {
            point_index: i as u32,
            value: DecimalNumber::try_from((*v).to_owned()).unwrap(),
        })
        .collect()
}
#[test]
fn normalized_ratios_preserve_extreme_exponents_sign_policy_and_zero_total() {
    let r = ratios(
        &weights(&["1e-4096", "1e4096"]),
        NegativeWeights::Reject,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(r.denominator.len(), 8193);
    assert_eq!(r.numerators[0].numerator, "1");
    assert_eq!(
        r.numerators[1].numerator,
        "1".to_owned() + &"0".repeat(8192)
    );
    assert!(r.denominator.ends_with('1'));
    assert!(
        ratios(
            &weights(&["-2.5", "5"]),
            NegativeWeights::Reject,
            Default::default(),
            &|| false
        )
        .is_err()
    );
    let r = ratios(
        &weights(&["-2.5", "5"]),
        NegativeWeights::AbsoluteMagnitude,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(r.denominator, "75");
    assert_eq!(r.numerators[0].numerator, "25");
    let r = ratios(
        &weights(&["-0", "0"]),
        NegativeWeights::Reject,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert!(r.zero_total);
    assert_eq!(r.denominator, "0");
}
#[test]
fn normalized_weights_share_geometry_preflight_budgets_and_cancellation() {
    let w = weights(&["1e-100", "1e100"]);
    assert!(
        ratios(
            &w,
            NegativeWeights::Reject,
            SectorLimits {
                max_scaled_decimal_digits: 100,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert!(
        ratios(
            &w,
            NegativeWeights::Reject,
            SectorLimits {
                max_boundary_decimal_digits: 100,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert!(ratios(&w, NegativeWeights::Reject, Default::default(), &|| true).is_err());
    let r = ratios(&w, NegativeWeights::Reject, Default::default(), &|| false).unwrap();
    let g = layout(
        &SectorRequest {
            start_turn: mo_geometry::Fixed::ZERO,
            direction: SectorDirection::Clockwise,
            negative_weights: NegativeWeights::Reject,
            weights: w,
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(g.work, r.work);
}
