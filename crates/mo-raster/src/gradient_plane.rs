//! A finite two-dimensional gradient field, separate from its color ramp.
//! Producers evaluate native gradient semantics before entering this layer.
use crate::{RasterError, RasterViewport, number::Scale};
use mo_geometry::{Fixed, Point};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GradientAxisTile {
    Clamp,
    Repeat,
    Mirror,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GradientPlane {
    /// World Q32 EMU position of the unit tile's top-left corner.
    pub origin: Point,
    /// World displacement for a unit change in each tile coordinate.
    pub x_step: Point,
    pub y_step: Point,
    pub tile_x: GradientAxisTile,
    pub tile_y: GradientAxisTile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Box<GradientPlaneUncertainty>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GradientPlaneUncertainty {
    /// Nonnegative Q32 world EMU error per input parameter.
    pub origin: Point,
    pub x_step: Point,
    pub y_step: Point,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum GradientField {
    /// First closed ellipse membership for q=((2u-1)*scaleX,(2v-1)*scaleY).
    /// At t=0 the ellipse has innerCenter and innerRadii; at t=1 it is the
    /// unit circle at the origin. Non-nested families use the first entry.
    Elliptic {
        /// Each dimensionless scale is in (0,1].
        #[serde(rename = "tileScale")]
        tile_scale: [Fixed; 2],
        #[serde(rename = "innerCenter")]
        inner_center: [Fixed; 2],
        #[serde(rename = "innerRadii")]
        inner_radii: [Fixed; 2],
        /// Nonnegative Q32 errors ordered scaleX/Y, centerX/Y, radiusX/Y.
        /// They bound geometry parameters, not a uniform scalar/color error.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uncertainty: Option<Box<[Fixed; 6]>>,
    },
    /// t = coefficients[0] * u + coefficients[1] * v + coefficients[2].
    /// u,v are the independently tiled/clamped coordinates in [0,1].
    Linear {
        coefficients: [Fixed; 3],
        /// Nonnegative dimensionless Q32 errors, before float conversion.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uncertainty: Option<[Fixed; 3]>,
    },
    /// Maximum of four inward edge fields and zero. Rates are ordered left,
    /// top, right, bottom; a zero rate disables that edge. Positive rates give
    /// 1-u*left, 1-v*top, 1-(1-u)*right and 1-(1-v)*bottom respectively.
    Rectangular {
        #[serde(rename = "edgeRates")]
        edge_rates: [Fixed; 4],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uncertainty: Option<[Fixed; 4]>,
    },
}

pub(crate) struct CompiledPlane {
    pub kind: u32,
    pub words: Vec<u32>,
    pub coordinate_error: Fixed,
    pub value_error: f64,
    pub elliptic: Option<crate::EllipticGradientWork>,
}

pub(crate) fn compile(
    plane: &GradientPlane,
    field: &GradientField,
    view: &RasterViewport,
) -> Result<CompiledPlane, RasterError> {
    let zero = Point {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
    };
    let e = plane
        .uncertainty
        .as_deref()
        .copied()
        .unwrap_or(GradientPlaneUncertainty {
            origin: zero,
            x_step: zero,
            y_step: zero,
        });
    let input = [
        e.x_step.x, e.y_step.x, e.origin.x, e.x_step.y, e.y_step.y, e.origin.y,
    ];
    if input.iter().any(|v| v.raw() < 0) {
        return Err(RasterError::Invalid("negative gradient plane uncertainty"));
    }
    let mut errors = [0; 6];
    for (out, v) in errors.iter_mut().zip(input) {
        *out = v
            .ratio_up(view.scale.numerator, view.scale.denominator)
            .map_err(|_| RasterError::Range)?
            .raw();
    }
    let (matrix, errors) =
        crate::paint_matrix::compile(plane.origin, plane.x_step, plane.y_step, errors, view)?;
    let repeated = |v| matches!(v, GradientAxisTile::Repeat | GradientAxisTile::Mirror);
    let error = crate::paint_precision::bound(
        matrix,
        errors,
        [0.0, 0.0, 1.0, 1.0],
        [0; 4],
        [1, 1],
        [repeated(plane.tile_x), repeated(plane.tile_y)],
        view,
    )?;
    crate::paint_matrix::bounds(matrix, [0.0, 0.0, 1.0, 1.0])?;
    let tile = |v| match v {
        GradientAxisTile::Clamp => 0,
        GradientAxisTile::Repeat => 1,
        GradientAxisTile::Mirror => 2,
    };
    let mut words = matrix.map(f32::to_bits).to_vec();
    words.extend([tile(plane.tile_x), tile(plane.tile_y)]);
    if let GradientField::Elliptic {
        tile_scale,
        inner_center,
        inner_radii,
        uncertainty,
    } = field
    {
        let (parameters, work) = crate::gradient_elliptic::compile(
            [
                tile_scale[0],
                tile_scale[1],
                inner_center[0],
                inner_center[1],
                inner_radii[0],
                inner_radii[1],
            ],
            uncertainty.as_deref().copied().unwrap_or([Fixed::ZERO; 6]),
            matrix,
            errors,
            error,
            view,
        )?;
        words.extend(parameters.map(f32::to_bits));
        return Ok(CompiledPlane {
            kind: 4,
            words,
            coordinate_error: work.coordinate_error_bound,
            value_error: 0.0,
            elliptic: Some(work),
        });
    }
    let (kind, coefficients, uncertainty): (u32, &[Fixed], &[Fixed]) = match field {
        GradientField::Elliptic { .. } => unreachable!("handled above"),
        GradientField::Linear {
            coefficients,
            uncertainty,
        } => (
            2,
            coefficients,
            uncertainty.as_ref().unwrap_or(&[Fixed::ZERO; 3]),
        ),
        GradientField::Rectangular {
            edge_rates,
            uncertainty,
        } => {
            if edge_rates.iter().any(|v| v.raw() < 0) {
                return Err(RasterError::Invalid("negative rectangular gradient rate"));
            }
            (
                3,
                edge_rates,
                uncertainty.as_ref().unwrap_or(&[Fixed::ZERO; 4]),
            )
        }
    };
    let unit = Scale::new(crate::PixelScale {
        numerator: 1,
        denominator: 1,
    })?;
    let mut value_error: f64 = 0.0;
    for (v, e) in coefficients.iter().zip(uncertainty) {
        if e.raw() < 0 {
            return Err(RasterError::Invalid("negative gradient field uncertainty"));
        }
        if kind == 3 && e.raw() != 0 && v.raw() <= e.raw() {
            return Err(RasterError::Precision);
        }
        let (f, conversion_error) = if kind == 3 {
            crate::gradient_rate::convert(v.raw())?
        } else {
            let f = unit.value(v.raw())?;
            (f, unit.error(v.raw(), f)?)
        };
        let raw = conversion_error
            .checked_add(e.raw())
            .ok_or(RasterError::Range)?;
        // The linear field sums coefficient errors. The maximum of edge fields
        // is 1-Lipschitz in the infinity norm, so its error is their maximum.
        if raw != 0 {
            let mut next = if kind == 3 {
                ((raw as f64).next_up() / 4294967296.0).next_up()
            } else {
                (raw as f64 / 4294967296.0).next_up()
            };
            if kind == 3 {
                // max(0,1-u*k) is active only for u <= 1/min(k,k').
                // This relative bound preserves narrow focus margins rather
                // than applying a device-coordinate range to dimensionless k.
                let lower = (f64::from(f) - next).next_down();
                next = (next / lower.max(1.0)).next_up();
            }
            value_error = if kind == 3 {
                value_error.max(next)
            } else {
                (value_error + next).next_up()
            };
        }
        words.push(f.to_bits());
    }
    if value_error > 1.0 / 1048576.0 {
        return Err(RasterError::Precision);
    }
    Ok(CompiledPlane {
        kind,
        words,
        coordinate_error: error,
        value_error,
        elliptic: None,
    })
}
