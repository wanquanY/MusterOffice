use crate::{
    Brush, GradientAlpha, GradientGeometry, GradientInterpolation, GradientTile, RasterError,
    RasterViewport, cancel, number::Scale,
};
use mo_geometry::Fixed;
use std::collections::BTreeMap;

pub(crate) const MAX_GRADIENTS: usize = 4096;
pub(crate) const MAX_STOPS: usize = 65536;
// Do not silently trigger Skia's <=2^-15 device-space degeneracy substitution.
const MIN_SPAN: f64 = 1.0 / 16384.0;
const VALUE_TOLERANCE: f64 = 1.0 / 1048576.0;

#[derive(Default)]
pub(crate) struct Gradients {
    pub words: Vec<Vec<u32>>,
    indices: BTreeMap<Vec<u32>, u32>,
    input_stops: usize,
    pub stored_stops: u32,
    pub coordinate_error: Fixed,
    pub value_error: f64,
    pub has_planes: bool,
    pub has_office_gamma: bool,
    pub has_rectangular: bool,
    pub elliptic: Option<crate::EllipticGradientWork>,
}
impl Gradients {
    /// Returns a solid word and 1-based gradient index (zero for a solid).
    pub fn insert(
        &mut self,
        brush: &Brush,
        view: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(u32, u32), RasterError> {
        cancel(check)?;
        let Brush::Gradient { gradient: g } = brush else {
            let Brush::Solid { rgba } = brush else {
                return Err(RasterError::Invalid("non-gradient brush"));
            };
            return Ok((u32::from_le_bytes(*rgba), 0));
        };
        self.input_stops = crate::paint_budget::stops(self.input_stops, g.stops.len())?;
        if !(2..=4096).contains(&g.stops.len()) {
            return Err(RasterError::Invalid("gradient stop count"));
        }
        if g.interpolation == GradientInterpolation::OfficeGamma1875 {
            if g.alpha != GradientAlpha::Straight || !crate::office_gamma_eligible(&g.stops) {
                return Err(RasterError::Invalid("Office gradient ramp eligibility"));
            }
            self.has_office_gamma = true;
        }
        let scale = Scale::new(view.scale)?;
        let mut point = |raw: i128| -> Result<u32, RasterError> {
            let v = scale.value(raw)?;
            let error = Fixed::from_raw(scale.error(raw, v)?);
            if error > view.coordinate_tolerance {
                return Err(RasterError::Precision);
            }
            self.coordinate_error = self.coordinate_error.max(error);
            Ok(v.to_bits())
        };
        let delta = |a: Fixed, b: Fixed| a.raw().checked_sub(b.raw()).ok_or(RasterError::Range);
        let (kind, coords) = match &g.geometry {
            GradientGeometry::Linear { start, end } => (
                0,
                vec![
                    point(delta(start.x, view.origin.x)?)?,
                    point(delta(start.y, view.origin.y)?)?,
                    point(delta(end.x, view.origin.x)?)?,
                    point(delta(end.y, view.origin.y)?)?,
                ],
            ),
            GradientGeometry::Radial { center, radius } => {
                if radius.raw() <= 0 {
                    return Err(RasterError::Invalid("gradient radius"));
                }
                (
                    1,
                    vec![
                        point(delta(center.x, view.origin.x)?)?,
                        point(delta(center.y, view.origin.y)?)?,
                        point(radius.raw())?,
                        0,
                    ],
                )
            }
            GradientGeometry::Plane { plane, field } => {
                let c = crate::gradient_plane::compile(plane, field, view)?;
                self.coordinate_error = self.coordinate_error.max(c.coordinate_error);
                self.value_error = self.value_error.max(c.value_error);
                self.has_planes = true;
                self.has_rectangular |= c.kind == 3;
                if let Some(work) = c.elliptic {
                    if let Some(prior) = &mut self.elliptic {
                        for i in 0..6 {
                            prior.parameter_error_bounds[i] =
                                prior.parameter_error_bounds[i].max(work.parameter_error_bounds[i]);
                        }
                        prior.coordinate_error_bound = prior
                            .coordinate_error_bound
                            .max(work.coordinate_error_bound);
                    } else {
                        self.elliptic = Some(work);
                    }
                }
                (c.kind, c.words)
            }
        };
        let f: Vec<_> = coords
            .iter()
            .copied()
            .map(|x| f64::from(f32::from_bits(x)))
            .collect();
        let span2 = if kind == 0 {
            (f[2] - f[0]).powi(2) + (f[3] - f[1]).powi(2)
        } else {
            f[2] * f[2]
        };
        if kind < 2 && span2 < MIN_SPAN * MIN_SPAN {
            return Err(RasterError::Precision);
        }
        let mut words = Vec::with_capacity(5 + coords.len() + 5 * g.stops.len());
        words.extend_from_slice(&[
            kind,
            match g.tile {
                GradientTile::Clamp => 0,
                GradientTile::Repeat => 1,
                GradientTile::Mirror => 2,
                GradientTile::Decal => 3,
            },
            match g.interpolation {
                GradientInterpolation::Srgb => 0,
                GradientInterpolation::LinearSrgb => 1,
                GradientInterpolation::OfficeGamma1875 => 2,
            },
            u32::from(g.alpha == GradientAlpha::Premultiplied),
            g.stops.len() as u32,
        ]);
        words.extend_from_slice(&coords);
        let mut previous: Option<(f64, f32)> = None;
        for stop in &g.stops {
            cancel(check)?;
            if !stop.position.is_finite() || !(0.0..=1.0).contains(&stop.position) {
                return Err(RasterError::Invalid("gradient stop position"));
            }
            let position = stop.position as f32;
            if let Some((p, q)) = previous {
                if stop.position < p {
                    return Err(RasterError::Invalid("gradient stop order"));
                }
                if stop.position > p && position <= q {
                    return Err(RasterError::Precision);
                }
            }
            // Distinct positions must not collapse into an implicit endpoint.
            if (stop.position > 0.0 && position == 0.0) || (stop.position < 1.0 && position == 1.0)
            {
                return Err(RasterError::Precision);
            }
            previous = Some((stop.position, position));
            for (i, value) in std::iter::once(stop.position).chain(stop.srgb).enumerate() {
                if !value.is_finite() || (i == 4 && !(0.0..=1.0).contains(&value)) {
                    return Err(RasterError::Invalid("gradient color"));
                }
                if value.abs() > 65504.0 {
                    return Err(RasterError::Invalid("gradient color range"));
                }
                let quantized = value as f32;
                let error = (value - f64::from(quantized)).abs();
                if error > VALUE_TOLERANCE {
                    return Err(RasterError::Precision);
                }
                self.value_error = self.value_error.max(error);
                words.push(if quantized == 0.0 {
                    0
                } else {
                    quantized.to_bits()
                });
            }
        }
        if let Some(index) = self.indices.get(&words) {
            return Ok((0, *index));
        }
        if self.words.len() == MAX_GRADIENTS
            || self.stored_stops as usize + g.stops.len() > MAX_STOPS
        {
            return Err(RasterError::Limit("gradient resources"));
        }
        self.stored_stops += g.stops.len() as u32;
        let index = self.words.len() as u32 + 1;
        self.indices.insert(words.clone(), index);
        self.words.push(words);
        Ok((0, index))
    }
}
