//! Sampled properties at the render boundary. Geometric channels use bounded
//! arithmetic; visibility stays discrete. Source declarations remain immutable.
use crate::{CompileError, angle::Angle, interval::Interval};
use mo_common::ObjectId;
use mo_timeline::{ExactValue, FrameState, MAX_SCALE_MILLI_PERCENT};
use num_bigint::BigInt;
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub(crate) struct SampledProperties {
    pub rotation: Option<SampledRotation>,
    pub scale: Option<[Interval; 2]>,
    pub visibility: Option<mo_timeline::Visibility>,
    pub opacity: Option<u16>,
    pub motion: Option<[Interval; 2]>,
}
#[derive(Clone)]
pub(crate) struct SampledRotation {
    angle: Angle,
    basis: mo_timeline::RotationBasis,
}
impl SampledRotation {
    pub fn resolve(&self, layout: Angle) -> Result<Angle, CompileError> {
        match self.basis {
            mo_timeline::RotationBasis::Absolute => Ok(self.angle.clone()),
            mo_timeline::RotationBasis::Layout => layout.compose(&self.angle, false),
        }
    }
}
pub(crate) fn sampled(
    state: &FrameState,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<ObjectId, SampledProperties>, CompileError> {
    let mut values = BTreeMap::<ObjectId, SampledProperties>::new();
    for (id, value) in &state.rotations {
        crate::cancel(check)?;
        values.entry(id.clone()).or_default().rotation = Some(SampledRotation {
            angle: Angle::exact(&value.value())?,
            basis: value.basis,
        });
    }
    for (id, value) in &state.scales {
        crate::cancel(check)?;
        values.entry(id.clone()).or_default().scale = Some([scale(&value.x)?, scale(&value.y)?]);
    }
    for (id, value) in &state.visibility {
        crate::cancel(check)?;
        values.entry(id.clone()).or_default().visibility = Some(*value);
    }
    for (id, value) in &state.motion {
        crate::cancel(check)?;
        values.entry(id.clone()).or_default().motion = Some([motion(&value.x)?, motion(&value.y)?]);
    }
    for (id, value) in &state.opacity {
        crate::cancel(check)?;
        values.entry(id.clone()).or_default().opacity = Some(opacity(value)?);
    }
    Ok(values)
}
fn motion(value: &ExactValue) -> Result<Interval, CompileError> {
    if value.numerator.len() > 2470 || value.denominator.len() > 2470 {
        return Err(CompileError::Limit("motion digits"));
    }
    let n = value
        .numerator
        .parse::<BigInt>()
        .map_err(|_| CompileError::Invalid("exact motion"))?;
    let d = value
        .denominator
        .parse::<BigInt>()
        .map_err(|_| CompileError::Invalid("exact motion"))?;
    if n.bits() > 8192 || d.bits() > 8192 {
        return Err(CompileError::Limit("exact motion arithmetic"));
    }
    if d <= BigInt::from(0) || n <= -(&d * 1_000_000_000u32) || n >= &d * 1_000_000_000u32 {
        return Err(CompileError::Invalid("motion range"));
    }
    let n = n << 96usize;
    let q = &n / &d;
    let remainder = &n % &d;
    let lo = &q - if remainder < BigInt::from(0) { 1 } else { 0 };
    let hi = &q + if remainder > BigInt::from(0) { 1 } else { 0 };
    Ok(Interval::raw(lo, hi))
}
impl SampledProperties {
    pub(crate) fn motion_emu(&self, size: mo_presentation_model::Size) -> Option<[Interval; 2]> {
        self.motion.as_ref().map(|v| {
            [
                v[0].mul(&Interval::integer(size.width.get())),
                v[1].mul(&Interval::integer(size.height.get())),
            ]
        })
    }
}
/// One nearest rounding at the compositing boundary; never per-fill alpha.
pub(crate) fn opacity(value: &ExactValue) -> Result<u16, CompileError> {
    if value.numerator.len() > 2470 || value.denominator.len() > 2470 {
        return Err(CompileError::Limit("opacity digits"));
    }
    let parse = |s: &str| {
        s.parse::<BigInt>()
            .map_err(|_| CompileError::Invalid("exact opacity"))
    };
    let n = parse(&value.numerator)?;
    let d = parse(&value.denominator)?;
    if n.bits() > 8192 || d.bits() > 8192 {
        return Err(CompileError::Limit("exact opacity arithmetic"));
    }
    if n < BigInt::from(0) || d <= BigInt::from(0) || n > d {
        return Err(CompileError::Invalid("opacity range"));
    }
    u16::try_from((n * 131070 + &d) / (d * 2)).map_err(|_| CompileError::Range)
}
fn scale(value: &ExactValue) -> Result<Interval, CompileError> {
    if value.numerator.len() > 2470 || value.denominator.len() > 2470 {
        return Err(CompileError::Limit("scale digits"));
    }
    let parse = |s: &str| {
        s.parse::<BigInt>()
            .map_err(|_| CompileError::Invalid("exact scale"))
    };
    let n = parse(&value.numerator)?;
    let d = parse(&value.denominator)?;
    if n.bits() > 8192 || d.bits() > 8192 {
        return Err(CompileError::Limit("exact scale arithmetic"));
    }
    if n < BigInt::from(0) || d <= BigInt::from(0) || n > &d * MAX_SCALE_MILLI_PERCENT {
        return Err(CompileError::Invalid("scale range"));
    }
    let d = d * 100000;
    let n = n << 96usize;
    let lo = &n / &d;
    let hi = &lo + if n % d == BigInt::from(0) { 0 } else { 1 };
    Ok(Interval::raw(lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_scale_boundary_is_outward_and_bounds_untrusted_internal_ratios() {
        let q = |n: &str, d: &str| {
            scale(&ExactValue {
                numerator: n.into(),
                denominator: d.into(),
            })
        };
        let third = q("100000", "3").unwrap();
        let unit = BigInt::from(1) << 96usize;
        assert!(third.lo.clone() * 3 <= unit && third.hi.clone() * 3 >= unit);
        assert_eq!(&third.hi - &third.lo, BigInt::from(1));
        for (n, d) in [
            ("-1", "1"),
            ("1", "0"),
            ("1", "-1"),
            ("2147483626", "1"),
            ("x", "1"),
        ] {
            assert!(q(n, d).is_err());
        }
        assert!(q(&"9".repeat(2471), "1").is_err());
        assert_eq!(q("0", "7").unwrap().hi, BigInt::from(0));
    }
    #[test]
    fn opacity_is_rounded_once_and_rejects_invalid_exact_input() {
        let q = |n: &str, d: &str| {
            opacity(&ExactValue {
                numerator: n.into(),
                denominator: d.into(),
            })
        };
        assert_eq!(q("0", "1").unwrap(), 0);
        assert_eq!(q("1", "1").unwrap(), 65535);
        assert_eq!(q("1", "2").unwrap(), 32768);
        assert_eq!(q("1", "3").unwrap(), 21845);
        for (n, d) in [("2", "1"), ("-1", "1"), ("1", "0"), ("1", "-1"), ("x", "1")] {
            assert!(q(n, d).is_err());
        }
        assert!(q(&"9".repeat(2471), "1").is_err());
    }
    #[test]
    fn rounded_zero_is_not_a_certified_empty_fill() {
        let affine = mo_geometry::Affine {
            linear: [mo_geometry::Fixed::ZERO; 4],
            ..mo_geometry::Affine::IDENTITY
        };
        let mut error = crate::AffineUncertainty {
            linear: [mo_geometry::Fixed::ZERO; 4],
            translation: mo_geometry::Point {
                x: mo_geometry::Fixed::ZERO,
                y: mo_geometry::Fixed::ZERO,
            },
        };
        assert!(crate::placement_core::empty_fill(&affine, &error));
        error.linear[0] = mo_geometry::Fixed::from_raw(1);
        error.linear[3] = mo_geometry::Fixed::from_raw(1);
        assert!(!crate::placement_core::empty_fill(&affine, &error));
    }
}
