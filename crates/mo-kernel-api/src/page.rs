//! Author page compilation and rendering share one typed computation boundary.
use mo_common::{ObjectId, from_json_str};
use mo_presentation_compile::{CompileError, PageError};
pub use mo_presentation_compile::{
    PAGE_PROFILE, PageFeature, PagePlan, PageRasterInfo, PageRenderRequest,
};
use mo_presentation_model::{ValidationCode, ValidationReport};
use mo_raster::{RasterBackend, RasterError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PageFailureCode {
    InputInvalid,
    LimitExceeded,
    CoordinateRange,
    PrecisionExceeded,
    Cancelled,
    MappingNotImplemented,
    ComponentFailure,
    ComponentInvalid,
    HostFailure,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageFailure {
    pub code: PageFailureCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ValidationReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<PageFeature>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PageCompileResponse {
    Compiled { plan: Box<PagePlan> },
    Error { error: PageFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PageRasterResponse {
    Rendered { info: Box<PageRasterInfo> },
    Error { error: PageFailure },
}
fn basic(code: PageFailureCode, message: String) -> PageFailure {
    PageFailure {
        code,
        message,
        report: None,
        object: None,
        feature: None,
    }
}
pub(crate) fn failure(e: PageError) -> PageFailure {
    use PageFailureCode::*;
    let message = e.to_string();
    let code = match e {
        PageError::MissingThemeColor { object, .. } => {
            return PageFailure {
                object,
                feature: Some(PageFeature::MissingThemeColor),
                ..basic(InputInvalid, message)
            };
        }
        PageError::Unsupported { object, feature } => {
            return PageFailure {
                object,
                feature: Some(feature),
                ..basic(MappingNotImplemented, message)
            };
        }
        PageError::Placement(CompileError::Document(report)) => {
            let code = if report
                .issues
                .iter()
                .any(|i| i.code == ValidationCode::LimitExceeded)
            {
                LimitExceeded
            } else {
                InputInvalid
            };
            return PageFailure {
                report: Some(report),
                ..basic(code, message)
            };
        }
        PageError::Placement(CompileError::Invalid(_))
        | PageError::Raster(RasterError::Invalid(_)) => InputInvalid,
        PageError::Placement(CompileError::Range) | PageError::Raster(RasterError::Range) => {
            CoordinateRange
        }
        PageError::Placement(CompileError::Cancelled)
        | PageError::Raster(RasterError::Cancelled) => Cancelled,
        PageError::Raster(RasterError::Limit(_)) | PageError::Placement(CompileError::Limit(_)) => {
            LimitExceeded
        }
        PageError::Raster(RasterError::Precision) => PrecisionExceeded,
        PageError::Raster(RasterError::Component(_)) => ComponentFailure,
        PageError::Raster(RasterError::ComponentInvalid(_)) => ComponentInvalid,
        PageError::Raster(RasterError::Host(_)) => HostFailure,
    };
    basic(code, message)
}
fn parse(input: &str) -> Result<PageRenderRequest, PageFailure> {
    if input.len() > super::MAX_REQUEST_BYTES {
        return Err(basic(
            PageFailureCode::LimitExceeded,
            "page request bytes".into(),
        ));
    }
    from_json_str(input).map_err(|e| basic(PageFailureCode::InputInvalid, e.to_string()))
}
pub fn compile_page_json(input: &str, check: &dyn Fn() -> bool) -> String {
    let result = parse(input)
        .and_then(|q| mo_presentation_compile::compile_page(&q, check).map_err(failure));
    let response = match result {
        Ok(plan) => PageCompileResponse::Compiled {
            plan: Box::new(plan),
        },
        Err(error) => PageCompileResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded page plan")
}
/// The host owns binary pixels, cancellation and atomic artifact publication.
pub fn render_page_json(
    input: &str,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result = parse(input)
        .and_then(|q| mo_presentation_compile::render_page(&q, backend, check).map_err(failure));
    let (response, pixels) = match result {
        Ok(image) => (
            PageRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (PageRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("bounded page metadata"),
        pixels,
    )
}
