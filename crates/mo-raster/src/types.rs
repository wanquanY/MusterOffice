use mo_common::{ByteLength, Digest};
use mo_geometry::{Fixed, PathCommand, Point};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelScale {
    /// Positive rational pixels per EMU; normalized internally.
    pub numerator: u32,
    pub denominator: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RasterViewport {
    pub width: u32,
    pub height: u32,
    /// Q32 EMU. Subtracted before converting to device-space float32.
    pub origin: Point,
    pub scale: PixelScale,
    /// Raw Q32 pixels; 256..=2^24 (at most 1/256 pixel).
    pub coordinate_tolerance: Fixed,
    /// Straight sRGB RGBA8; output is premultiplied.
    pub background: [u8; 4],
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FillRule {
    Nonzero,
    Evenodd,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillPath {
    pub fill_rule: FillRule,
    /// Local Q32 EMU; open contours are implicitly closed for filling.
    pub commands: Vec<PathCommand>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum StrokeJoin {
    Miter {
        limit: Fixed,
    },
    /// Limit 1..=1024, dimensionless Q32. Clip perpendicular to the exterior
    /// bisector at half-width * limit. Exact reversals use a bevel join.
    MiterClip {
        limit: Fixed,
    },
    // Empty struct variants enforce deny_unknown_fields for internally tagged
    // JSON. Serde unit variants otherwise accept stray parameters silently.
    Round {},
    Bevel {},
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StrokeStyle {
    /// Q32 page/world EMU. Zero explicitly requests a device hairline.
    pub width: Fixed,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathDraw {
    #[serde(default, skip_serializing_if = "BlendMode::is_default")]
    pub blend: BlendMode,
    /// Last intersection node; None uses the viewport alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<u32>,
    pub path: u32,
    pub origin: Point,
    pub brush: crate::Brush,
    /// None fills; Some strokes the path, without implicitly closing contours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<StrokeStyle>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathRasterRequest {
    pub viewport: RasterViewport,
    pub paths: Vec<FillPath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<PathClip>,
    pub draws: Vec<PathDraw>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RasterWork {
    /// Only present for V12 ellipse fields. Geometry and solver errors are separate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elliptic_gradients: Option<EllipticGradientWork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compositing: Option<CompositeWork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clips: Option<ClipWork>,
    pub paths: u32,
    pub commands: u32,
    pub draws: u32,
    pub drawn_commands: u32,
    /// Conservative maximum error of the transformed control coordinates,
    /// measured in raw Q32 pixels. Not a raster coverage error bound.
    pub coordinate_error_bound: Fixed,
    pub stroke_styles: u32,
    pub stroke_draws: u32,
    /// Width quantization, in raw Q32 pixels; not an ink boundary error bound.
    pub stroke_width_error_bound: Fixed,
    /// Dimensionless raw Q32; not added to pixel coordinate budgets.
    pub miter_limit_error_bound: Fixed,
    pub gradients: u32,
    pub gradient_stops: u32,
    pub gradient_draws: u32,
    /// Device geometry input quantization, not a bound on shader evaluation or pixels.
    pub gradient_coordinate_error_bound: Fixed,
    /// Maximum stop/color conversion or linear/rectangular field error.
    /// Elliptic parameter and root bounds are reported separately.
    pub gradient_value_error_bound: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RasterInfo {
    pub profile: String,
    pub width: u32,
    pub height: u32,
    pub byte_length: ByteLength,
    pub sha256: Digest,
    /// SHA-256 of the little-endian device batch; use together with profile.
    pub frame_sha256: Digest,
    pub work: RasterWork,
}
pub struct RasterImage {
    pub info: RasterInfo,
    pub pixels: Vec<u8>,
}

/// A reusable intersection of a filled path with its parent or the viewport.
/// Empty paths clip everything. Origin is world Q32 EMU, independent of draws.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathClip {
    /// Parents must precede children. Maximum depth 64, maximum 8192 nodes.
    pub parent: Option<u32>,
    pub path: u32,
    pub origin: Point,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClipWork {
    /// All supplied clip path commands checked during device placement.
    pub placement_commands: u32,
    pub nodes: u32,
    pub maximum_depth: u32,
    /// Actual pushes when preserving common ancestors between consecutive draws.
    pub applications: u32,
    pub applied_commands: u32,
}

/// Porter-Duff composition, separate from the source brush and geometry mask.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BlendMode {
    #[default]
    SourceOver,
    Source,
}
impl BlendMode {
    pub fn is_default(&self) -> bool {
        *self == Self::SourceOver
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositeWork {
    pub captures: u32,
    /// Exact packed pixel bytes copied for all snapshots; not process RSS.
    pub captured_bytes: u32,
    pub snapshot_draws: u32,
    pub source_draws: u32,
}

/// Maximum across elliptic fields; excludes pixel coverage and shader inverse arithmetic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EllipticGradientWork {
    /// Dimensionless Q32 source uncertainty plus binary32 conversion error,
    /// ordered scaleX/Y, centerX/Y, radiusX/Y.
    pub parameter_error_bounds: [Fixed; 6],
    /// Conservative device Q32 displacement of corresponding ellipse-family
    /// points from parameter/plane conversion, including the tile phase budget.
    pub coordinate_error_bound: Fixed,
    /// Q32 width of a successful solver enclosure for the encoded binary32
    /// field. Not a bound on the source field's scalar or on pixel color.
    pub encoded_root_interval_bound: Fixed,
}
