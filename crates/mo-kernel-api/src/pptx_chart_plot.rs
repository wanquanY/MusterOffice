use super::{
    PptxChartGeometryFailure, PptxFailureCode, pptx_chart_geometry::failure,
    pptx_source::inline_limits,
};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use mo_presentation_compile::source_chart_plot;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use source_chart_plot::{SourceChartPlot, SourceChartPlotLimits, SourceChartPlotRequest};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxChartPlotResponse {
    Compiled { plot: Box<SourceChartPlot> },
    Error { error: PptxChartGeometryFailure },
}
pub fn compile_pptx_chart_plot<R: ReaderAt>(
    request: &SourceChartPlotRequest,
    reader: R,
    length: u64,
    limits: SourceLimits,
    plot_limits: SourceChartPlotLimits,
    check: &dyn Fn() -> bool,
) -> PptxChartPlotResponse {
    let result = (|| {
        let package =
            Package::open(reader, length, limits.package, check).map_err(PptxError::from)?;
        let index = inspect_source(&package, limits, check)?;
        source_chart_plot::compile(&package, &index, request, limits, plot_limits, check)
    })();
    match result {
        Ok(plot) => PptxChartPlotResponse::Compiled {
            plot: Box::new(plot),
        },
        Err(e) => PptxChartPlotResponse::Error { error: failure(e) },
    }
}
pub fn compile_pptx_chart_plot_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        PptxChartPlotResponse::Error {
            error: failure(PptxError::Limit("chart plot request bytes").into()),
        }
    } else {
        match from_json_str::<SourceChartPlotRequest>(input) {
            Ok(request) => compile_pptx_chart_plot(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                SourceChartPlotLimits::default(),
                &|| false,
            ),
            Err(e) => PptxChartPlotResponse::Error {
                error: PptxChartGeometryFailure {
                    code: PptxFailureCode::InputInvalid,
                    source_ordinal: None,
                    series_index: None,
                    point_index: None,
                    message: e.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("bounded compiled source chart plot")
}
