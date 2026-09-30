//! Native chart annotations share layout/paint/text declarations. These records
//! are source facts, not resolved label text, chart layout or editing promises.
use super::*;
use crate::source::{SourceRunKind, text::SourceTextCatalog};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartAnnotations {
    /// Physical source order. Parent ordinals refer to chart, plot, series, axis
    /// or another annotation; no synthetic shape/object IDs are allocated.
    pub nodes: Vec<SourceChartAnnotation>,
    /// One catalog per real c:txPr/c:rich body, including chart/axis defaults.
    pub text_bodies: Vec<SourceChartTextBody>,
}
impl SourceChartAnnotations {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.text_bodies.is_empty()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartAnnotationKind {
    DataLabels,
    DataLabel,
    Legend,
    LegendEntry,
    Title,
    Layout,
    ManualLayout,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartAnnotation {
    pub source_ordinal: u32,
    pub parent_ordinal: u32,
    pub kind: ChartAnnotationKind,
    /// Native idx for dLbl/legendEntry only; sparse values stay sparse.
    pub index: Option<u32>,
    pub declarations: SourceChartLayout,
    pub number_format: Option<SourceChartNumberFormat>,
    pub text_source: Option<SourceChartAnnotationText>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartAnnotationText {
    pub source_ordinal: u32,
    pub content: ChartAnnotationTextContent,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ChartAnnotationTextContent {
    Rich {
        source_ordinal: u32,
    },
    /// Formula/cache snapshot only. Does not open or recalculate a workbook.
    StringReference {
        channel: SourceChartChannel,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartTextBody {
    pub source_ordinal: u32,
    pub parent_ordinal: u32,
    pub styles: SourceTextCatalog,
    pub paragraphs: Vec<SourceChartTextParagraph>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartTextParagraph {
    pub source_ordinal: u32,
    pub runs: Vec<SourceChartTextRun>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartTextRun {
    pub source_ordinal: u32,
    pub text_source_ordinal: Option<u32>,
    pub kind: SourceRunKind,
    pub text: String,
}
