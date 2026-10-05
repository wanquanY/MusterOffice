use mo_common::RationalTime;
use mo_time::{ExactRatio as R, Rounding as Q, TimeError};
use proptest::prelude::*;

fn ratio(n: i64, d: i64) -> R {
    R::new(n.into(), d.into(), 256).unwrap()
}

#[test]
fn signed_rounding_and_even_ties_are_explicit() {
    for (n, d, floor, ceil, trunc, nearest) in [
        (1, 2, 0, 1, 0, 0),
        (3, 2, 1, 2, 1, 2),
        (5, 2, 2, 3, 2, 2),
        (-1, 2, -1, 0, 0, 0),
        (-3, 2, -2, -1, -1, -2),
        (-5, 2, -3, -2, -2, -2),
        (-7, 3, -3, -2, -2, -2),
        (7, 3, 2, 3, 2, 2),
        (0, 17, 0, 0, 0, 0),
    ] {
        let r = ratio(n, d);
        for (mode, expected) in [
            (Q::Floor, floor),
            (Q::Ceil, ceil),
            (Q::TowardZero, trunc),
            (Q::NearestTiesEven, nearest),
        ] {
            assert_eq!(
                r.round_integer(mode).unwrap(),
                expected,
                "{n}/{d}: {mode:?}"
            );
        }
    }
}
#[test]
fn preserves_unrepresentable_exact_values_until_explicit_boundary() {
    let r = ratio(1, 1_000_000_001);
    assert_eq!(r.to_time_exact(), Err(TimeError::Representation));
    let q = r.quantize(1_000_000, Q::NearestTiesEven, 256).unwrap();
    assert_eq!(q.value.ticks.get(), 0);
    assert_eq!(q.residual.compare(&r), std::cmp::Ordering::Equal);
    assert_eq!(r.quantize(0, Q::Floor, 256), Err(TimeError::Representation));
}
#[test]
fn reverse_and_error_states_are_not_silent_rounding() {
    assert_eq!(
        ratio(2, 3)
            .div(&ratio(-4, 5), 256)
            .unwrap()
            .compare(&ratio(-5, 6)),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        ratio(2, 3).div(&R::integer(0), 256),
        Err(TimeError::DivisionByZero)
    );
    assert_eq!(R::new(1.into(), 0.into(), 256), Err(TimeError::Denominator));
    assert_eq!(
        R::new(1.into(), (-1).into(), 256),
        Err(TimeError::Denominator)
    );
    assert_eq!(ratio(127, 1).mul(&ratio(127, 1), 7), Err(TimeError::Budget));
    assert_eq!(R::new(1.into(), 1.into(), 4097), Err(TimeError::Budget));
    assert_eq!(
        R::integer(i64::MAX)
            .add(&R::integer(1), 256)
            .unwrap()
            .to_time_exact(),
        Err(TimeError::Representation)
    );
}
#[test]
fn hour_of_fractional_frames_uses_absolute_audio_and_microsecond_boundaries() {
    let frames = 107_892i64;
    let mut last_sample = 0;
    let mut total_samples = 0;
    for n in 1..=frames {
        let time = R::frame_time(n, 30_000, 1_001, 256).unwrap();
        let sample = time
            .mul(&R::integer(48_000), 256)
            .unwrap()
            .round_integer(Q::Floor)
            .unwrap();
        assert!(matches!(sample - last_sample, 1601 | 1602));
        total_samples += sample - last_sample;
        last_sample = sample;
        let q = time.quantize(1_000_000, Q::NearestTiesEven, 256).unwrap();
        assert!(q.residual.compare(&ratio(1, 2_000_000)).is_le());
        assert!(q.residual.compare(&ratio(-1, 2_000_000)).is_ge());
    }
    assert_eq!(total_samples, 172_799_827);
    assert_eq!(
        R::frame_time(frames, 30_000, 1_001, 256)
            .unwrap()
            .to_time_exact()
            .unwrap(),
        RationalTime::new(8_999_991, 2_500).unwrap()
    );
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]
    #[test]
    fn addition_and_inverse_match_independent_integer_cross_products(a in -1_000_000i64..1_000_000,b in 1i64..1_000_000,c in -1_000_000i64..1_000_000,d in 1i64..1_000_000) {
        let lhs=ratio(a,b);let rhs=ratio(c,d);let sum=lhs.add(&rhs,256).unwrap();
        let n=i128::try_from(sum.numerator()).unwrap();let den=i128::try_from(sum.denominator()).unwrap();
        prop_assert_eq!(n*i128::from(b)*i128::from(d),(i128::from(a)*i128::from(d)+i128::from(c)*i128::from(b))*den);
        prop_assert_eq!(sum.sub(&rhs,256).unwrap().compare(&lhs),std::cmp::Ordering::Equal);
    }
}
