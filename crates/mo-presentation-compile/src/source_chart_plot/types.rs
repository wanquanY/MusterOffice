use crate::{chart_geometry::ChartGeometryLimits, source_chart::SourceCircularRequest};
use mo_common::Digest;
use mo_geometry::Fixed;
use mo_presentation_source::source::{
    SourceColorMapRef, SourceObjectRef,
    charts::{
        ChartDataAuthority, SourceChartExternalData,
        paints::{ChartPaintLimits, ChartThemeOverride},
        styles::ChartStyleLimits,
    },
    color::ColorContext,
    line::resolve::EffectiveLineGeometry,
    theme::SourceThemeSchemeRef,
};
use mo_render::DrawScene;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum ChartPlotProfile {
    #[serde(rename = "source-circular-declared-solid-plot-v1-draft")]
    DeclaredSolidDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPlotRequest {
    pub profile: ChartPlotProfile,
    pub geometry: SourceCircularRequest,
    pub color_context: ColorContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPlot {
    pub profile: ChartPlotProfile,
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    pub chart_part: String,
    pub chart_sha256: Digest,
    pub plot_source_ordinal: u32,
    pub native_kind: String,
    pub data_authority: ChartDataAuthority,
    pub external_data: Option<SourceChartExternalData>,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
    pub theme_override: Option<ChartThemeOverride>,
    /// Maximum source geometry error, Q32 EMU. Raster consumers must compose
    /// this with their own transform/device error before accepting a frame.
    pub coordinate_error_bound: Fixed,
    pub scene: DrawScene,
    pub series: Vec<ChartPlotSeries>,
    pub points: Vec<ChartPlotPoint>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartPlotSeries {
    pub index: u32,
    pub order: u32,
    pub source_ordinal: u32,
    pub values_source_ordinal: u32,
    pub formula: Option<String>,
    pub inner_radius: Fixed,
    pub outer_radius: Fixed,
    pub coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartPlotPoint {
    pub series_index: u32,
    pub point_index: u32,
    pub cache_source_ordinal: u32,
    /// None for zero-valued points; stable point identity is still retained.
    pub path: Option<u32>,
    pub fill: ChartPlotPaint,
    pub line: ChartPlotPaint,
    pub line_geometry: EffectiveLineGeometry,
    pub effects_source_ordinal: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ChartPlotPaint {
    None {
        declaration: u32,
    },
    Solid {
        declaration: u32,
        color: u32,
        rgba8: [u8; 4],
    },
}
#[derive(Debug, Clone, Copy)]
pub struct SourceChartPlotLimits {
    pub paints: ChartPaintLimits,
    pub geometry: ChartGeometryLimits,
    pub styles: ChartStyleLimits,
    pub max_draws: usize,
}
impl Default for SourceChartPlotLimits {
    fn default() -> Self {
        Self {
            paints: Default::default(),
            geometry: Default::default(),
            styles: Default::default(),
            max_draws: 8192,
        }
    }
}
