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
pub use mo_presentation_compile::source_chart::{
    SourceCircularGeometry, SourceCircularLimits, SourceCircularRequest,
};
use mo_presentation_compile::{
    chart_geometry::ChartGeometryError,
    source_chart::{self, SourceCircularError},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxChartGeometryResponse {
    Compiled {
        geometry: Box<SourceCircularGeometry>,
    },
    Error {
        error: PptxChartGeometryFailure,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxChartGeometryFailure {
    pub code: PptxFailureCode,
    pub source_ordinal: Option<u32>,
    pub series_index: Option<u32>,
    pub point_index: Option<u32>,
    pub message: String,
}
fn failure(error: SourceCircularError) -> PptxChartGeometryResponse {
    use SourceCircularError as E;
    let mut failure = PptxChartGeometryFailure {
        code: PptxFailureCode::MappingNotImplemented,
        source_ordinal: None,
        series_index: None,
        point_index: None,
        message: error.to_string(),
    };
    match error {
        E::Source(error) => {
            let source = pptx_failure(error);
            failure.code = source.code;
            failure.message = source.message;
        }
        E::Unresolved {
            source_ordinal,
            series_index,
            point_index,
            ..
        } => {
            failure.source_ordinal = Some(source_ordinal);
            failure.series_index = series_index;
            failure.point_index = point_index;
        }
        E::Geometry(error) => {
            use mo_charts::sectors::SectorError as S;
            failure.code = match &error {
                ChartGeometryError::Cancelled | ChartGeometryError::Sectors(S::Cancelled) => {
                    PptxFailureCode::Cancelled
                }
                ChartGeometryError::Limit(_) | ChartGeometryError::Sectors(S::Limit(_)) => {
                    PptxFailureCode::LimitExceeded
                }
                ChartGeometryError::Precision | ChartGeometryError::Range => {
                    PptxFailureCode::MappingNotImplemented
                }
                _ => PptxFailureCode::InputInvalid,
            };
            if let ChartGeometryError::Sectors(S::NegativeWeight(i) | S::DuplicatePoint(i)) = error
            {
                failure.point_index = Some(i);
            }
        }
    }
    PptxChartGeometryResponse::Error { error: failure }
}

pub fn compile_pptx_chart_geometry<R: ReaderAt>(
    request: &SourceCircularRequest,
    reader: R,
    length: u64,
    limits: SourceLimits,
    chart_limits: SourceCircularLimits,
    check: &dyn Fn() -> bool,
) -> PptxChartGeometryResponse {
    let result = (|| {
        let package =
            Package::open(reader, length, limits.package, check).map_err(PptxError::from)?;
        let index = inspect_source(&package, limits, check)?;
        source_chart::compile(&package, &index, request, limits, chart_limits, check)
    })();
    match result {
        Ok(geometry) => PptxChartGeometryResponse::Compiled {
            geometry: Box::new(geometry),
        },
        Err(e) => failure(e),
    }
}
pub fn compile_pptx_chart_geometry_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        failure(PptxError::Limit("chart geometry request bytes").into())
    } else {
        match from_json_str::<SourceCircularRequest>(input) {
            Ok(request) => compile_pptx_chart_geometry(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                SourceCircularLimits::default(),
                &|| false,
            ),
            Err(error) => PptxChartGeometryResponse::Error {
                error: PptxChartGeometryFailure {
                    code: PptxFailureCode::InputInvalid,
                    source_ordinal: None,
                    series_index: None,
                    point_index: None,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("bounded source circular geometry")
}
