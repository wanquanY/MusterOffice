//! Device-space bounds for image parameter uncertainty, including tile phase.
//! The interval calculations below are a verifier, never a sampling algorithm.
use crate::{RasterError as E, RasterViewport};
use mo_geometry::Fixed;

const Q: f64 = 4294967296.0;

// f32 parameters are exact in f64. Every subsequent operation encloses its
// result with adjacent representable numbers; integer-to-float conversions are
// enclosed too. There is no unproved epsilon in the inverse/period calculation.
#[derive(Clone, Copy)]
pub(crate) struct I {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}
impl I {
    pub(crate) fn exact(v: f64) -> Self {
        Self { lo: v, hi: v }
    }
    pub(crate) fn integer(v: i128) -> Self {
        let f = v as f64;
        if v.unsigned_abs() <= (1u128 << 53) {
            Self::exact(f)
        } else {
            Self {
                lo: f.next_down(),
                hi: f.next_up(),
            }
        }
    }
    pub(crate) fn around(v: f64, raw_error: i128) -> Self {
        if raw_error == 0 {
            return Self::exact(v);
        }
        let e = Self::integer(raw_error).hi / Q;
        Self {
            lo: (v - e).next_down(),
            hi: (v + e).next_up(),
        }
    }
    pub(crate) fn zero(self) -> bool {
        self.lo == 0.0 && self.hi == 0.0
    }
    pub(crate) fn add(self, b: Self) -> Self {
        if self.zero() {
            return b;
        }
        if b.zero() {
            return self;
        }
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    pub(crate) fn sub(self, b: Self) -> Self {
        self.add(Self {
            lo: -b.hi,
            hi: -b.lo,
        })
    }
    pub(crate) fn mul(self, b: Self) -> Self {
        if self.zero() || b.zero() {
            return Self::exact(0.0);
        }
        let values = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        Self {
            lo: values.into_iter().fold(f64::INFINITY, f64::min).next_down(),
            hi: values
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
    pub(crate) fn div(self, b: Self) -> Result<Self, E> {
        if b.lo <= 0.0 && b.hi >= 0.0 {
            return Err(E::Precision);
        }
        Ok(self.mul(Self {
            lo: (1.0 / b.hi).next_down(),
            hi: (1.0 / b.lo).next_up(),
        }))
    }
    pub(crate) fn magnitude(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
}

pub(crate) fn ceil_integer(v: f64) -> Result<i128, E> {
    // Far below i128's conversion/saturation boundary. Larger extents cannot
    // meet the public <= one-pixel budget with any nonzero Q32 error anyway.
    if !v.is_finite() || !(0.0..=2.0f64.powi(90)).contains(&v) {
        return Err(E::Precision);
    }
    Ok(v.ceil() as i128)
}
pub(crate) fn bound(
    matrix: [f32; 6],
    errors: [i128; 6],
    domain: [f32; 4],
    domain_errors: [i128; 4],
    mut extent: [i128; 2],
    repeated: [bool; 2],
    view: &RasterViewport,
) -> Result<Fixed, E> {
    if errors.iter().chain(&domain_errors).all(|v| *v == 0) {
        return Ok(Fixed::ZERO);
    }
    let m: [I; 6] = std::array::from_fn(|i| I::around(f64::from(matrix[i]), errors[i]));
    let d: [I; 4] = std::array::from_fn(|i| I::around(f64::from(domain[i]), domain_errors[i]));
    let periods = [d[2].sub(d[0]), d[3].sub(d[1])];
    if periods.iter().any(|p| p.lo <= 0.0) {
        return Err(E::Precision);
    }
    let det = m[0].mul(m[4]).sub(m[1].mul(m[3]));
    if det.lo <= 0.0 && det.hi >= 0.0 {
        return Err(E::Precision);
    }
    let mut phase_extent = [0.0f64; 2];
    if repeated.into_iter().any(|v| v) {
        let mut inverse_extent = [0.0f64; 2];
        // Every affine in the uncertainty box is nonsingular above. Interval
        // evaluation contains both source and encoded inverses at each corner;
        // affine coordinates attain their viewport extrema at these corners.
        for x in [-1.0, f64::from(view.width) + 1.0] {
            for y in [-1.0, f64::from(view.height) + 1.0] {
                let x = I::exact(x).sub(m[2]);
                let y = I::exact(y).sub(m[5]);
                let source = [
                    m[4].mul(x).sub(m[1].mul(y)).div(det)?,
                    m[0].mul(y).sub(m[3].mul(x)).div(det)?,
                ];
                for i in 0..2 {
                    inverse_extent[i] = inverse_extent[i].max(source[i].magnitude());
                    phase_extent[i] = phase_extent[i].max(source[i].sub(d[i]).magnitude());
                }
            }
        }
        for i in 0..2 {
            if repeated[i] {
                extent[i] = extent[i].max(ceil_integer(inverse_extent[i])?);
            }
        }
    }
    let mut source_error = [I::exact(0.0); 2];
    for i in 0..2 {
        let lo = domain_errors[i];
        let hi = domain_errors[i + 2];
        source_error[i] = I::integer(lo.max(hi));
        if repeated[i] && (lo != 0 || hi != 0) {
            // Mirror may reverse either edge. Two additional periods cover
            // endpoint neighborhoods on both sides of the viewport.
            let count = I::exact(phase_extent[i]).div(periods[i])?.hi.ceil();
            source_error[i] = source_error[i].add(
                I::exact(count)
                    .add(I::exact(2.0))
                    .mul(I::exact(2.0))
                    .mul(I::integer(lo).add(I::integer(hi))),
            );
        }
    }
    let mut result = 0;
    for row in [0, 3] {
        let affine = errors[row]
            .checked_mul(extent[0])
            .and_then(|v| {
                errors[row + 1]
                    .checked_mul(extent[1])
                    .and_then(|w| v.checked_add(w))
            })
            .and_then(|v| v.checked_add(errors[row + 2]))
            .ok_or(E::Range)?;
        // The uncertain source matrix, not just its rounded coefficients,
        // maps domain errors; this includes the coefficient/domain cross term.
        let domain = I::exact(m[row].magnitude())
            .mul(source_error[0])
            .add(I::exact(m[row + 1].magnitude()).mul(source_error[1]));
        let bound = affine
            .checked_add(ceil_integer(domain.hi)?)
            .ok_or(E::Range)?;
        if bound > view.coordinate_tolerance.raw() {
            return Err(E::Precision);
        }
        result = result.max(bound);
    }
    Ok(Fixed::from_raw(result))
}
