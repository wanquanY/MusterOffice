use crate::{
    ShapeVariation,
    lines::{LineShapeRequest, LineShapeResult},
    metrics::{FontMetric, MeasuredInstance},
};
use mo_common::Emu;
use mo_geometry::Fixed;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryStyle {
    pub font_size: Emu,
    /// Positive values raise the baseline in the y-down output coordinate system.
    pub baseline_shift: BaselineShift,
    /// Extra visual trailing advance per shaped cluster, including the final
    /// cluster on a line. Never inserted between glyphs in the same cluster.
    #[serde(default, skip_serializing_if = "zero")]
    pub cluster_spacing: Fixed,
}
fn zero(value: &Fixed) -> bool {
    *value == Fixed::ZERO
}
/// Keep legacy integer-EMU inputs while allowing native percentages to reach
/// layout without an intermediate integer-EMU rounding. Equality is numeric.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum BaselineShift {
    Emu(Emu),
    Q32 { q32: Fixed },
}
impl BaselineShift {
    pub fn position(self) -> Fixed {
        match self {
            Self::Emu(v) => Fixed::emu(v),
            Self::Q32 { q32 } => q32,
        }
    }
}
impl From<Emu> for BaselineShift {
    fn from(value: Emu) -> Self {
        Self::Emu(value)
    }
}
impl PartialEq for BaselineShift {
    fn eq(&self, other: &Self) -> bool {
        self.position() == other.position()
    }
}
impl Eq for BaselineShift {}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum LineSpacing {
    Natural,
    Exact {
        height: Emu,
    },
    AtLeast {
        height: Emu,
    },
    /// One nonnegative Q32 height per geometry style. Each line uses the
    /// maximum among its text items; a text-free line uses the strut style.
    /// Native percentage policies are resolved once by the source compiler.
    StyleMaximum {
        heights: Vec<Fixed>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineGeometryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<super::LeftTabStops>,
    pub shaping: LineShapeRequest,
    /// One geometry style for each paragraph shaping style.
    pub styles: Vec<GeometryStyle>,
    /// Explicit minimum font/size participant, including otherwise empty lines.
    pub strut_style: u32,
    pub spacing: LineSpacing,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricInstance {
    pub font: u32,
    pub variations: Vec<ShapeVariation>,
    pub position_units_per_em: u32,
    pub measured: MeasuredInstance,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FragmentRef {
    pub fallback_item: u32,
    pub fragment: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PositionedGlyph {
    pub source: FragmentRef,
    pub glyph: u32,
    pub x: Emu,
    pub y: Emu,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryLine {
    pub top: Emu,
    pub baseline: Emu,
    pub bottom: Emu,
    pub height: Emu,
    /// Signed horizontal pen displacement; not ink/visual bounds.
    pub advance: Emu,
    pub advance_y: Emu,
    pub pen_min: Emu,
    pub pen_max: Emu,
    pub visual_fragments: Vec<FragmentRef>,
    pub removed_by_x9: Vec<FragmentRef>,
    pub glyphs: Vec<PositionedGlyph>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryLayout {
    pub height: Emu,
    pub lines: Vec<GeometryLine>,
}
/// Same evaluated line boxes/pen bounds before the legacy integer-EMU wire
/// conversion. Frame placement must accumulate these, not rounded wire values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreciseLineGeometry {
    pub top: Fixed,
    pub baseline: Fixed,
    pub bottom: Fixed,
    pub height: Fixed,
    pub advance: Fixed,
    pub advance_y: Fixed,
    pub pen_min: Fixed,
    pub pen_max: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreciseGeometryLayout {
    pub height: Fixed,
    pub lines: Vec<PreciseLineGeometry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum GeometryIssue {
    UnresolvedFont { start: u32, end: u32 },
    MissingMetric { instance: u32, metric: FontMetric },
    Tab { start: u32, end: u32 },
    MixedLevelCluster { start: u32, end: u32 },
    NonPositiveNaturalHeight { line: u32 },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineGeometryResult {
    pub profile: String,
    pub shaping: LineShapeResult,
    pub metric_instances: Vec<MetricInstance>,
    /// None when any layout prerequisite is unresolved; no partial geometry.
    pub layout: Option<GeometryLayout>,
    pub issues: Vec<GeometryIssue>,
}
