//! Source numeric conversion; no binary floating point or guessed font scaling.
use super::SourceTextError;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_presentation_source::source::drawingml::NativePercentage;
use mo_text::geometry::BaselineShift;

pub(super) fn tracking(
    value: &mo_presentation_source::source::text::NativeTextPoint,
) -> Result<(Fixed, Fixed), SourceTextError> {
    use crate::source_number::{PercentageError, decimal};
    use mo_presentation_source::source::text::NativeTextPoint;
    let scaled = match value {
        NativeTextPoint::HundredthPoints { value } => {
            return Ok((Fixed::emu(Emu::new(i64::from(*value) * 127)), Fixed::ZERO));
        }
        NativeTextPoint::UniversalMeasure { value } => {
            let s = value.lexical();
            if s.len() > 256 {
                return Err(SourceTextError::Limit("native tracking lexical bytes"));
            }
            let (number, factor) = [
                ("mm", 36000),
                ("cm", 360000),
                ("in", 914400),
                ("pt", 12700),
                ("pc", 152400),
                ("pi", 152400),
            ]
            .into_iter()
            .find_map(|(unit, factor)| s.strip_suffix(unit).map(|n| (n, factor)))
            .ok_or(SourceTextError::Invalid("native tracking unit"))?;
            decimal(number, factor, 1).map_err(|e| match e {
                PercentageError::LexicalLimit => {
                    SourceTextError::Limit("native tracking lexical bytes")
                }
                PercentageError::Range => SourceTextError::Invalid("native tracking range"),
            })?
        }
    };
    Ok((
        scaled.value,
        if scaled.fractional {
            Fixed::from_raw(1)
        } else {
            Fixed::ZERO
        },
    ))
}

pub(super) fn baseline(
    value: &NativePercentage,
    size: Emu,
) -> Result<BaselineShift, SourceTextError> {
    use crate::source_number::{PercentageError, percentage};
    let scaled = percentage(value, size).map_err(|e| match e {
        PercentageError::LexicalLimit => SourceTextError::Limit("native baseline lexical bytes"),
        PercentageError::Range => SourceTextError::Invalid("native baseline range"),
    })?;
    if scaled.fractional {
        Ok(BaselineShift::Q32 { q32: scaled.value })
    } else {
        scaled
            .value
            .wire()
            .map(Into::into)
            .map_err(|_| SourceTextError::Invalid("native baseline range"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(s: &str, size: i64) -> BaselineShift {
        baseline(&s.to_owned().try_into().unwrap(), Emu::new(size)).unwrap()
    }
    #[test]
    fn exact_percentage_units_signed_fractions_and_ties_reach_q32() {
        assert_eq!(value("30000", 381000), value("30%", 381000));
        assert_eq!(
            value("30000", 381000).position(),
            Fixed::emu(Emu::new(114300))
        );
        assert_eq!(
            value("-25000", 381000).position(),
            Fixed::emu(Emu::new(-95250))
        );
        for sign in ["", "-"] {
            let raw = if sign.is_empty() { 1 } else { -1 };
            assert_eq!(
                value(&format!("{sign}0.0000000116415321826934814453125%"), 1)
                    .position()
                    .raw(),
                raw
            );
            assert_eq!(
                value(&format!("{sign}0.0000000116415321826934814453124%"), 1)
                    .position()
                    .raw(),
                0
            );
        }
        assert_eq!(value("12.345678901%", 127).position().raw(), 67340844651);
        assert!(matches!(value("0", 127), BaselineShift::Emu(_)));
        assert!(matches!(
            value("0.0000000000000000000001%", 127),
            BaselineShift::Q32 { .. }
        ));
    }
    #[test]
    fn numeric_and_lexical_bounds_are_checked_before_layout() {
        let large: NativePercentage = ("9".repeat(70) + "%").try_into().unwrap();
        assert!(matches!(
            baseline(&large, Emu::new(381000)),
            Err(SourceTextError::Invalid("native baseline range"))
        ));
        let long: NativePercentage = ("0.".to_owned() + &"0".repeat(260) + "%")
            .try_into()
            .unwrap();
        assert!(matches!(
            baseline(&long, Emu::new(381000)),
            Err(SourceTextError::Limit("native baseline lexical bytes"))
        ));
    }
    #[test]
    fn tracking_units_signed_fractions_and_conversion_bounds_are_exact() {
        use mo_presentation_source::source::text::NativeTextPoint as P;
        let unit = |s: &str| P::UniversalMeasure {
            value: s.to_owned().try_into().unwrap(),
        };
        assert_eq!(
            tracking(&P::HundredthPoints { value: 100 }).unwrap(),
            tracking(&unit("1pt")).unwrap()
        );
        assert_eq!(
            tracking(&unit("2.54cm")).unwrap(),
            tracking(&unit("1in")).unwrap()
        );
        assert_eq!(
            tracking(&unit("12pt")).unwrap(),
            tracking(&unit("1pc")).unwrap()
        );
        assert_eq!(
            tracking(&unit("1pi")).unwrap(),
            tracking(&unit("1pc")).unwrap()
        );
        assert_eq!(
            tracking(&unit("-0.001mm")).unwrap().0,
            Fixed::emu(Emu::new(-36))
        );
        let (v, e) = tracking(&unit("0.00001pt")).unwrap();
        assert_eq!(v.raw(), 545460847);
        assert_eq!(e, Fixed::from_raw(1));
        let (_, e) = tracking(&unit("0.000000000000000000000001pt")).unwrap();
        assert_eq!(e, Fixed::from_raw(1));
        assert!(matches!(
            tracking(&unit(&("0.".to_owned() + &"0".repeat(260) + "pt"))),
            Err(SourceTextError::Limit("native tracking lexical bytes"))
        ));
        assert!(matches!(
            tracking(&unit(&("9".repeat(70) + "pt"))),
            Err(SourceTextError::Invalid("native tracking range"))
        ));
    }
}
