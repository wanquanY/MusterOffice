use super::{
    PptxFailure, PptxFailureCode,
    pptx_source::{inline_limits, pptx_failure},
};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, charts::paints, inspect_source},
};
pub use paints::{ChartPaintLimits, SourceChartPaintQuery, SourceChartPaints};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxChartPaintResponse {
    Computed { paints: Box<SourceChartPaints> },
    Error { error: PptxFailure },
}
pub fn compute_pptx_chart_paints<R: ReaderAt>(
    request: &SourceChartPaintQuery,
    reader: R,
    length: u64,
    limits: SourceLimits,
    paint_limits: ChartPaintLimits,
    check: &dyn Fn() -> bool,
) -> PptxChartPaintResponse {
    let result = (|| {
        let package = Package::open(reader, length, limits.package, check)?;
        let index = inspect_source(&package, limits, check)?;
        paints::query(&package, &index, request, limits, paint_limits, check)
    })();
    match result {
        Ok(paints) => PptxChartPaintResponse::Computed {
            paints: Box::new(paints),
        },
        Err(error) => PptxChartPaintResponse::Error {
            error: pptx_failure(error),
        },
    }
}
pub fn compute_pptx_chart_paints_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        PptxChartPaintResponse::Error {
            error: pptx_failure(PptxError::Limit("chart paint request bytes")),
        }
    } else {
        match from_json_str::<SourceChartPaintQuery>(input) {
            Ok(request) => compute_pptx_chart_paints(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                ChartPaintLimits::default(),
                &|| false,
            ),
            Err(error) => PptxChartPaintResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("bounded chart paint response")
}
