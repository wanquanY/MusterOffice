use super::{
    PptxFailureCode,
    pptx_source::{inline_limits, pptx_failure},
};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use mo_presentation_compile::source_chart_labels::{self, ChartLabelError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use source_chart_labels::{SourceChartLabelLimits, SourceChartLabelRequest, SourceChartLabels};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxChartLabelsResponse {
    Planned { labels: Box<SourceChartLabels> },
    Error { error: PptxChartLabelFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxChartLabelFailure {
    pub code: PptxFailureCode,
    pub source_ordinal: Option<u32>,
    pub series_index: Option<u32>,
    pub point_index: Option<u32>,
    pub message: String,
}
fn failure(error: ChartLabelError) -> PptxChartLabelFailure {
    let mut f = PptxChartLabelFailure {
        code: PptxFailureCode::MappingNotImplemented,
        source_ordinal: None,
        series_index: None,
        point_index: None,
        message: error.to_string(),
    };
    match error {
        ChartLabelError::Source(e) => {
            let p = pptx_failure(e);
            f.code = p.code;
            f.message = p.message;
        }
        ChartLabelError::Unresolved {
            source_ordinal,
            series_index,
            point_index,
            ..
        } => {
            f.source_ordinal = Some(source_ordinal);
            f.series_index = series_index;
            f.point_index = point_index;
        }
        ChartLabelError::Ratios(e) => {
            use mo_charts::sectors::SectorError as E;
            f.code = match e {
                E::Cancelled => PptxFailureCode::Cancelled,
                E::Limit(_) => PptxFailureCode::LimitExceeded,
                _ => PptxFailureCode::InputInvalid,
            };
            if let E::NegativeWeight(i) | E::DuplicatePoint(i) = e {
                f.point_index = Some(i);
            }
        }
        ChartLabelError::NumberFormat(e) => {
            use mo_charts::number_format::FormatError as E;
            f.code = match e {
                E::Cancelled => PptxFailureCode::Cancelled,
                E::Limit(_) => PptxFailureCode::LimitExceeded,
                E::Unresolved(_) => PptxFailureCode::MappingNotImplemented,
                E::Symbols | E::Ratio => PptxFailureCode::InputInvalid,
            };
        }
    }
    f
}
pub fn compute_pptx_chart_labels<R: ReaderAt>(
    request: &SourceChartLabelRequest,
    reader: R,
    length: u64,
    limits: SourceLimits,
    label_limits: SourceChartLabelLimits,
    check: &dyn Fn() -> bool,
) -> PptxChartLabelsResponse {
    let result = (|| {
        let package =
            Package::open(reader, length, limits.package, check).map_err(PptxError::from)?;
        let index = inspect_source(&package, limits, check)?;
        source_chart_labels::compute(&package, &index, request, limits, label_limits, check)
    })();
    match result {
        Ok(labels) => PptxChartLabelsResponse::Planned {
            labels: Box::new(labels),
        },
        Err(e) => PptxChartLabelsResponse::Error { error: failure(e) },
    }
}
pub fn compute_pptx_chart_labels_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        PptxChartLabelsResponse::Error {
            error: failure(PptxError::Limit("chart label request bytes").into()),
        }
    } else {
        match from_json_str::<SourceChartLabelRequest>(input) {
            Ok(request) => compute_pptx_chart_labels(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                Default::default(),
                &|| false,
            ),
            Err(e) => PptxChartLabelsResponse::Error {
                error: PptxChartLabelFailure {
                    code: PptxFailureCode::InputInvalid,
                    source_ordinal: None,
                    series_index: None,
                    point_index: None,
                    message: e.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("bounded chart label plan")
}
