//! Native linear, rectangular and circle gradient geometry and working colors. Coordinates belong to
//! the receiving shape, including inherited properties, never the donor box.
use super::*;
use crate::source_number::{percentage_interval, percentage_ratio};
use mo_presentation_model::Size;
use mo_presentation_source::source::color::ColorSample;
use mo_raster::{
    Gradient, GradientAlpha, GradientAxisTile, GradientField, GradientGeometry,
    GradientInterpolation, GradientPlane, GradientPlaneUncertainty, GradientStop, GradientTile,
};
type E = SourcePageError;
pub(super) fn numeric(error: crate::CompileError) -> E {
    if matches!(error, crate::CompileError::Cancelled) {
        RasterError::Cancelled.into()
    } else {
        RasterError::Precision.into()
    }
}
pub(super) fn enclose(v: Fixed, error: Fixed) -> I {
    let v = I::fixed(v);
    let e = I::fixed(error);
    I::raw(v.lo - e.lo, v.hi + e.hi)
}
fn point(v: [I; 2]) -> Result<(Point, Point), E> {
    let (x, ex) = v[0].q32().map_err(numeric)?;
    let (y, ey) = v[1].q32().map_err(numeric)?;
    Ok((Point { x, y }, Point { x: ex, y: ey }))
}
fn abs(v: &I) -> I {
    let zero = BigInt::from(0);
    if v.hi <= zero {
        v.neg()
    } else if v.lo >= zero {
        v.clone()
    } else {
        I::raw(zero, (-&v.lo).max(v.hi.clone()))
    }
}
/// Nearest binary64, ties to even, for a bounded decimal ratio in [0,1].
/// No preliminary Q32 rounding: distinct author hard stops must remain distinct.
fn position(n: &BigInt, d: &BigInt) -> Result<f64, E> {
    if n < &BigInt::from(0) || n > d {
        return Err(E::Invalid("gradient stop outside unit interval"));
    }
    if n == &BigInt::from(0) {
        return Ok(0.0);
    }
    let mut e = n.bits() as i32 - d.bits() as i32;
    if (n << (-e) as usize) < *d {
        e -= 1;
    }
    // Native decimal lexical budget proves e >= -867. Keep this precondition
    // explicit rather than saturating or losing a tiny endpoint to zero.
    if e < -969 {
        return Err(RasterError::Precision.into());
    }
    let numerator = n << (52 - e) as usize;
    let mut q = &numerator / d;
    let remainder = (&numerator % d) * 2;
    if remainder > *d || remainder == *d && &q % 2 != BigInt::from(0) {
        q += 1;
    }
    let q = u64::try_from(q).map_err(|_| RasterError::Range)?;
    let value = q as f64 * f64::from_bits(((e - 52 + 1023) as u64) << 52);
    if n < d && value >= 1.0 {
        return Err(RasterError::Precision.into());
    }
    Ok(value)
}
pub(super) fn compile(
    result: &SourceFillColorResult,
    placement: Option<&NativePlacement>,
    page_size: Size,
    radial: Option<&crate::radial_layout::NativeRadialLayout>,
    check: &dyn Fn() -> bool,
) -> Result<Gradient, E> {
    cancel(check)?;
    let FillOutcome::Resolved { fill, .. } = &result.style else {
        return Err(E::Invalid("gradient style unresolved"));
    };
    let EffectiveFill::Gradient { gradient: g, .. } = fill.as_ref() else {
        return Err(E::Invalid("gradient color/style mismatch"));
    };
    let FillPaintColors::Gradient { stops: colors } = &result.colors else {
        return Err(E::Invalid("gradient colors missing"));
    };
    if placement.is_some() && !g.rotate_with_shape.value {
        return Err(E::Invalid(
            "native stationary gradient orientation required",
        ));
    }
    let (tile_origin, w, h, field) = if let Some(layout) = radial {
        super::gradient_circle::geometry(layout)?
    } else {
        let size = placement.map_or(page_size, |p| p.source_size);
        let width = I::integer(size.width.get());
        let height = I::integer(size.height.get());
        let r = &g.tile_rect;
        let pct = |v| {
            percentage_interval(v)
                .map_err(|_| E::Invalid("gradient percentage lexical limit or range"))
        };
        let l = pct(&r.left.value)?;
        let t = pct(&r.top.value)?;
        let w = width.mul(&I::integer(1).sub(&l).sub(&pct(&r.right.value)?));
        let h = height.mul(&I::integer(1).sub(&t).sub(&pct(&r.bottom.value)?));
        if w.lo <= BigInt::from(0) || h.lo <= BigInt::from(0) {
            return Err(E::Invalid(
                "gradient tile rectangle must have positive extents",
            ));
        }
        let field = match &g.shade {
            EffectiveGradientShade::Linear { angle, scaled, .. } => {
                let [cos, sin] =
                    crate::trig::cos_sin(i64::from(angle.value), check).map_err(numeric)?;
                // Corrected scaled normal is (h*cos, w*sin) in physical coordinates.
                // In unit tile coordinates its common w*h factor cancels exactly.
                let (nx, ny) = if scaled.value {
                    (cos, sin)
                } else {
                    (cos.mul(&w), sin.mul(&h))
                };
                let denominator = abs(&nx).add(&abs(&ny));
                let offset = abs(&nx).sub(&nx).add(&abs(&ny).sub(&ny)).divide(2);
                let mut coefficients = [Fixed::ZERO; 3];
                let mut coefficient_error = coefficients;
                for (i, c) in [nx, ny, offset].iter().enumerate() {
                    (coefficients[i], coefficient_error[i]) = c
                        .div_positive(&denominator)
                        .map_err(numeric)?
                        .q32()
                        .map_err(numeric)?;
                }
                GradientField::Linear {
                    coefficients,
                    uncertainty: Some(coefficient_error),
                }
            }
            EffectiveGradientShade::Path {
                path, fill_to_rect, ..
            } if path.value == mo_presentation_source::source::fill::NativePathShade::Rectangle => {
                super::gradient_rect::field(fill_to_rect)?
            }
            _ => return Err(E::Invalid("native path gradient evaluation required")),
        };
        ([width.mul(&l), height.mul(&t)], w, h, field)
    };
    let (affine, linear_error, translation_error, anchor) = match placement {
        Some(p) => (
            p.affine,
            p.uncertainty.linear,
            p.uncertainty.translation,
            Point {
                x: p.anchor.x.checked_sub(Fixed::emu(p.source_origin.x))?,
                y: p.anchor.y.checked_sub(Fixed::emu(p.source_origin.y))?,
            },
        ),
        None => (Affine::IDENTITY, [Fixed::ZERO; 4], ZERO, ZERO),
    };
    let m: [I; 4] = std::array::from_fn(|i| enclose(affine.linear[i], linear_error[i]));
    let local = [
        tile_origin[0].sub(&I::fixed(anchor.x)),
        tile_origin[1].sub(&I::fixed(anchor.y)),
    ];
    let (origin, origin_error) = point([
        m[0].mul(&local[0])
            .add(&m[1].mul(&local[1]))
            .add(&enclose(affine.translation.x, translation_error.x)),
        m[2].mul(&local[0])
            .add(&m[3].mul(&local[1]))
            .add(&enclose(affine.translation.y, translation_error.y)),
    ])?;
    let (x_step, x_error) = point([m[0].mul(&w), m[2].mul(&w)])?;
    let (y_step, y_error) = point([m[1].mul(&h), m[3].mul(&h)])?;
    if g.stops.value.len() != colors.len() || !(2..=4096).contains(&colors.len()) {
        return Err(E::Invalid("gradient stop cardinality"));
    }
    let mut stops = Vec::with_capacity(colors.len());
    let mut previous: Option<(BigInt, BigInt, f64)> = None;
    for (stop, color) in g.stops.value.iter().zip(colors) {
        cancel(check)?;
        let ColorSample::Resolved { srgb, .. } = color.outcome else {
            return Err(E::Invalid("gradient color unresolved"));
        };
        let (n, d) = percentage_ratio(&stop.position.value)
            .map_err(|_| E::Invalid("gradient stop lexical limit or range"))?;
        let p = position(&n, &d)?;
        if let Some((pn, pd, pp)) = &previous {
            let ordering = (&n * pd).cmp(&(pn * &d));
            if ordering.is_lt() {
                return Err(E::Invalid("gradient stop order"));
            }
            if ordering.is_gt() && p <= *pp {
                return Err(RasterError::Precision.into());
            }
        }
        previous = Some((n, d, p));
        stops.push(GradientStop { position: p, srgb });
    }
    let interpolation = if mo_raster::office_gamma_eligible(&stops) {
        GradientInterpolation::OfficeGamma1875
    } else {
        GradientInterpolation::Srgb
    };
    Ok(Gradient {
        geometry: GradientGeometry::Plane {
            plane: GradientPlane {
                origin,
                x_step,
                y_step,
                // MS-OI29500 2.1.1297: Office ignores gradFill.flip, using xy.
                // The source declaration and provenance remain in the binding.
                tile_x: GradientAxisTile::Mirror,
                tile_y: GradientAxisTile::Mirror,
                uncertainty: Some(Box::new(GradientPlaneUncertainty {
                    origin: origin_error,
                    x_step: x_error,
                    y_step: y_error,
                })),
            },
            field,
        },
        stops: stops.into(),
        tile: GradientTile::Clamp,
        // Office's explicitly documented two-stop / symmetric three-stop rule.
        // Eligibility is evaluated on source values, not quantized stop words.
        interpolation,
        alpha: GradientAlpha::Straight,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn near_endpoint_source_position_cannot_change_interpolation_class() {
        let denominator = BigInt::from(10).pow(100);
        assert!(position(&(&denominator - 1), &denominator).is_err());
        assert_eq!(position(&denominator, &denominator).unwrap(), 1.0);
    }
    #[test]
    fn decimal_stops_round_once_and_keep_ties_even() {
        for (n, d, expected) in [
            (0, 1, 0.0),
            (1, 1, 1.0),
            (1, 10, 0.1),
            (7, 10, 0.7),
            (1, 3, 1.0 / 3.0),
        ] {
            assert_eq!(
                position(&BigInt::from(n), &BigInt::from(d)).unwrap(),
                expected
            );
        }
        let denominator = BigInt::from(1) << 54usize;
        assert_eq!(
            position(&((BigInt::from(1) << 53usize) + 1), &denominator).unwrap(),
            0.5
        );
        assert_eq!(
            position(&((BigInt::from(1) << 53usize) + 3), &denominator).unwrap(),
            0.5f64.next_up().next_up()
        );
        assert!(position(&BigInt::from(-1), &BigInt::from(2)).is_err());
    }
}
