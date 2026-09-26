use crate::{
    geometry::{GeometryStyle, LineGeometryResult, LineSpacing},
    paragraph::ParagraphShapeRequest,
};
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_unicode::{TextBoundary, line_break::LineBreakAnalysis};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OverflowPolicy {
    KeepUnbreakable,
    EmergencyGrapheme,
}

/// Exact available widths, independent of source format. First means the first
/// visual line of this paragraph, not the first line after every explicit break.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineWidths {
    pub first: Fixed,
    pub rest: Fixed,
}
impl LineWidths {
    pub fn uniform(width: Fixed) -> Self {
        Self {
            first: width,
            rest: width,
        }
    }
    pub(crate) fn at(self, start: u32) -> Fixed {
        if start == 0 { self.first } else { self.rest }
    }
}

/// Borrowed computation input for a native frame compiler. The legacy wire
/// request converts to this same input; there is only one line-search engine.
pub struct FlowInput<'a> {
    pub paragraph: &'a ParagraphShapeRequest,
    pub styles: &'a [GeometryStyle],
    pub strut_style: u32,
    pub spacing: LineSpacing,
    pub widths: LineWidths,
    pub overflow: OverflowPolicy,
}
impl<'a> From<&'a ParagraphLayoutRequest> for FlowInput<'a> {
    fn from(q: &'a ParagraphLayoutRequest) -> Self {
        Self {
            paragraph: &q.paragraph,
            styles: &q.styles,
            strut_style: q.strut_style,
            spacing: q.spacing.clone(),
            widths: LineWidths::uniform(Fixed::emu(q.width)),
            overflow: q.overflow,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphLayoutRequest {
    pub paragraph: ParagraphShapeRequest,
    pub styles: Vec<GeometryStyle>,
    pub strut_style: u32,
    pub spacing: LineSpacing,
    pub width: Emu,
    pub overflow: OverflowPolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum FlowIssue {
    Tab { scalar: u32 },
    ConditionalHyphen { scalar: u32 },
    ContingentObject { scalar: u32 },
    MixedLevelCluster { start: u32, end: u32 },
    UnresolvedFont { start: u32, end: u32 },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowWork {
    pub evaluated_candidates: u32,
    pub candidate_scalars: u32,
    pub verified_faces: u32,
    pub shaping_runs: u32,
    pub component_calls: u32,
    pub context_scalars: u32,
    pub probed_glyphs: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineDecision {
    pub end: TextBoundary,
    pub emergency: bool,
    pub overflows: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphLayoutResult {
    pub profile: String,
    pub breaks: LineBreakAnalysis,
    pub suppressed_grapheme_breaks: Vec<TextBoundary>,
    /// Empty if prerequisites prevent a complete plan. A terminal explicit
    /// line break has one final empty line at the same source end coordinate.
    pub decisions: Vec<LineDecision>,
    pub geometry: Option<LineGeometryResult>,
    pub issues: Vec<FlowIssue>,
    pub work: FlowWork,
}
