//! Evaluated world-space paint. Author fill inheritance and gradient geometry
//! belong to the presentation compiler, not to the raster backend.
use crate::RasterError;
use mo_geometry::{Fixed, Point};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Brush {
    Solid {
        rgba: [u8; 4],
    },
    Gradient {
        gradient: Gradient,
    },
    Image {
        image: crate::ImageBrush,
    },
    /// Immutable full-viewport pixels before draw `after_draws`, after group
    /// boundaries at that position. Device-aligned; path/view transforms do not
    /// move it. Scope selects a canvas, never an implicit host resource.
    Snapshot {
        #[serde(rename = "afterDraws")]
        after_draws: u32,
        #[serde(default, skip_serializing_if = "SnapshotScope::is_current")]
        scope: SnapshotScope,
    },
}
/// The source canvas at a capture point. Explicit captures may be consumed
/// later in any group; their pixels remain immutable after the source closes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SnapshotScope {
    /// Legacy local capture: use must stay in its original innermost group.
    #[default]
    Current,
    /// Root output, excluding any unmerged active group surfaces.
    Output,
    /// Zero-based opacity-group index; must be active at the capture point.
    Group { index: u32 },
}
impl SnapshotScope {
    pub fn is_current(&self) -> bool {
        *self == Self::Current
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gradient {
    pub geometry: GradientGeometry,
    /// Nondecreasing positions in [0,1]. Repeated positions are hard stops.
    /// Missing endpoint positions extend the first/last color to that endpoint.
    pub stops: crate::GradientStops,
    pub tile: GradientTile,
    pub interpolation: GradientInterpolation,
    pub alpha: GradientAlpha,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum GradientGeometry {
    Linear {
        start: Point,
        end: Point,
    },
    Radial {
        center: Point,
        radius: Fixed,
    },
    Plane {
        plane: crate::GradientPlane,
        field: crate::GradientField,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GradientStop {
    pub position: f64,
    /// Straight sRGB working channels. Finite RGB may exceed [0,1]; alpha must
    /// be in [0,1]. No RGBA8 quantization is performed before interpolation.
    pub srgb: [f64; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GradientTile {
    Clamp,
    Repeat,
    Mirror,
    Decal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GradientInterpolation {
    Srgb,
    LinearSrgb,
    /// Office's endpoint-pair RGB curve, exponent 15/8; alpha stays linear.
    /// Requires two endpoint stops or a symmetric three-stop ramp, straight alpha.
    OfficeGamma1875,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GradientAlpha {
    Straight,
    Premultiplied,
}
impl Brush {
    /// Paint is already in world space and is independent of a path's local
    /// origin/transform. Scene lowering rebases it exactly once with the view.
    pub fn rebased(&self, origin: Point) -> Result<Self, RasterError> {
        let mut out = self.clone();
        let shift = |p: &mut Point| -> Result<(), RasterError> {
            p.x = p.x.checked_sub(origin.x).map_err(|_| RasterError::Range)?;
            p.y = p.y.checked_sub(origin.y).map_err(|_| RasterError::Range)?;
            Ok(())
        };
        if let Self::Gradient { gradient } = &mut out {
            match &mut gradient.geometry {
                GradientGeometry::Linear { start, end } => {
                    shift(start)?;
                    shift(end)?;
                }
                GradientGeometry::Radial { center, .. } => shift(center)?,
                GradientGeometry::Plane { plane, .. } => shift(&mut plane.origin)?,
            }
        }
        if let Self::Image { image } = &mut out {
            shift(&mut image.origin)?;
        }
        Ok(out)
    }
}
