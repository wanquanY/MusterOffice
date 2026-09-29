use super::{
    PptxFailure, PptxFailureCode,
    pptx_source::{inline_limits, pptx_failure},
};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::charts::{SourceChartLimits, SourceChartQuery, SourceCharts};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxChartsResponse {
    Inspected { charts: SourceCharts },
    Error { error: PptxFailure },
}
pub fn inspect_pptx_charts<R: ReaderAt>(
    request: &SourceChartQuery,
    reader: R,
    length: u64,
    limits: SourceLimits,
    chart_limits: SourceChartLimits,
    check: &dyn Fn() -> bool,
) -> PptxChartsResponse {
    let result = (|| {
        let package = Package::open(reader, length, limits.package, check)?;
        let index = inspect_source(&package, limits, check)?;
        mo_pptx::source::charts::query(&package, &index, request, limits, chart_limits, check)
    })();
    match result {
        Ok(charts) => PptxChartsResponse::Inspected { charts },
        Err(error) => PptxChartsResponse::Error {
            error: pptx_failure(error),
        },
    }
}
pub fn inspect_pptx_charts_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        PptxChartsResponse::Error {
            error: pptx_failure(PptxError::Limit("chart query request bytes")),
        }
    } else {
        match from_json_str::<SourceChartQuery>(input) {
            Ok(request) => inspect_pptx_charts(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                SourceChartLimits::default(),
                &|| false,
            ),
            Err(error) => PptxChartsResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed bounded source chart response")
}
