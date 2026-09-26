//! Nonnegative, dimensionless Q32 rates have a different range from device pixels.
use crate::{RasterError, number::float_interval};

pub(crate) fn convert(raw: i128) -> Result<(f32, i128), RasterError> {
    if raw < 0 {
        return Err(RasterError::Invalid("negative rectangular gradient rate"));
    }
    if raw == 0 {
        return Ok((0.0, 0));
    }
    let n = raw as u128;
    let mut exponent = 127 - n.leading_zeros() as i32 - 32;
    let shift = exponent + 9;
    let mut mantissa = if shift <= 0 {
        n << (-shift)
    } else {
        let d = 1u128 << shift;
        let (q, r) = (n / d, n % d);
        q + u128::from(r * 2 > d || r * 2 == d && q % 2 == 1)
    };
    if mantissa == 1 << 24 {
        mantissa >>= 1;
        exponent += 1;
    }
    // A rounding carry to 2^95 would put the Q32 float outside signed i128.
    if exponent >= 95 {
        return Err(RasterError::Range);
    }
    let f = f32::from_bits(((exponent + 127) as u32) << 23 | (mantissa as u32 - (1 << 23)));
    let (lo, hi) = float_interval(f);
    Ok((f, (lo - raw).abs().max((hi - raw).abs())))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensionless_rates_round_once_across_the_complete_fixed_domain() {
        for exponent in [-32, -9, 0, 15, 23, 64, 94] {
            let raw = 1i128 << (exponent + 32);
            let (value, error) = convert(raw).unwrap();
            assert_eq!(value, 2.0f32.powi(exponent));
            assert_eq!(error, 0);
        }
        let base = 1i128 << 96; // Rate 2^64, float step 2^41.
        let half = 1i128 << 72;
        assert_eq!(convert(base + half).unwrap().0, 2.0f32.powi(64));
        assert_eq!(
            convert(base + half + 1).unwrap().0,
            2.0f32.powi(64).next_up()
        );
        assert_eq!(
            convert(base + 3 * half).unwrap().0,
            2.0f32.powi(64).next_up().next_up()
        );
        assert!(convert(i128::MAX).is_err());
        assert!(convert(-1).is_err());
        assert_eq!(convert(0).unwrap(), (0.0, 0));
    }
}
