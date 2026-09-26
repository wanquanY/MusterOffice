//! Exact source percentage arithmetic shared by baseline and paragraph spacing.
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_pptx::source::drawingml::NativePercentage;
use num_bigint::BigInt;

pub(crate) enum PercentageError {
    LexicalLimit,
    Range,
}
pub(crate) struct ScaledNumber {
    pub value: Fixed,
    pub fractional: bool,
}
pub(crate) fn percentage(
    value: &NativePercentage,
    size: Emu,
) -> Result<ScaledNumber, PercentageError> {
    let s = value.lexical();
    if s.len() > 256 {
        return Err(PercentageError::LexicalLimit);
    }
    let (s, unit) = s.strip_suffix('%').map_or((s, 100_000), |s| (s, 100));
    decimal(s, size.get(), unit)
}
/// The source reader owns grammar validation. This function bounds work and
/// performs exact decimal scaling with one final Q32 rounding.
pub(crate) fn decimal(s: &str, factor: i64, unit: i32) -> Result<ScaledNumber, PercentageError> {
    let (n, d) = decimal_ratio(s, unit)?;
    let n = n * factor;
    let fractional = &n % &d != BigInt::from(0);
    let n = n << 32usize;
    let q = &n / &d;
    let remainder = &n % &d;
    let absolute = if remainder < BigInt::from(0) {
        -remainder
    } else {
        remainder
    };
    let q = if absolute * 2 >= d {
        q + if n < BigInt::from(0) { -1 } else { 1 }
    } else {
        q
    };
    let raw = i128::try_from(q).map_err(|_| PercentageError::Range)?;
    Ok(ScaledNumber {
        value: Fixed::from_raw(raw),
        fractional,
    })
}
/// Exact ratio before target precision is chosen. Source layout must not round
/// an input percentage to Q32 before dividing a potentially narrow rectangle.
pub(crate) fn decimal_ratio(s: &str, unit: i32) -> Result<(BigInt, BigInt), PercentageError> {
    if s.len() > 256 {
        return Err(PercentageError::LexicalLimit);
    }
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    let n = format!("{whole}{fraction}")
        .parse::<BigInt>()
        .map_err(|_| PercentageError::Range)?;
    let d = BigInt::from(unit) * BigInt::from(10).pow(fraction.len() as u32);
    Ok((n, d))
}

/// Ratio and outward interval before a consumer chooses target precision.
pub(crate) fn percentage_ratio(
    value: &NativePercentage,
) -> Result<(BigInt, BigInt), PercentageError> {
    let s = value.lexical();
    if s.len() > 256 {
        return Err(PercentageError::LexicalLimit);
    }
    let (s, unit) = s.strip_suffix('%').map_or((s, 100_000), |s| (s, 100));
    decimal_ratio(s, unit)
}
pub(crate) fn percentage_interval(
    value: &NativePercentage,
) -> Result<crate::interval::Interval, PercentageError> {
    let (n, d) = percentage_ratio(value)?;
    let n = n << 96usize;
    Ok(crate::interval::Interval::raw(
        crate::interval_extended::floor_ratio(&n, &d),
        -crate::interval_extended::floor_ratio(&(-n), &d),
    ))
}
