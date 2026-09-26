//! Inspect native radial focus geometry without calling host painting components.
use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_geometry::GeometryError;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, fill::resolve::SourceFillQuery, inspect_source},
};
pub use mo_presentation_compile::radial_layout::{
    NativeRadialLayout, RadialLayoutLimits, RadialLayoutOptions, SourceRadialLayoutPlan,
};
use mo_presentation_compile::{
    native_paths::NativePathError,
    radial_layout::{
        CircleFocusBasis, RadialLayoutError, SourceRadialLayoutError, layout_source_with_basis,
    },
    source_placement::SourcePlacementError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum RadialLayoutProfile {
    #[serde(rename = "drawingml-circle-path-bounds-q96-v1-draft")]
    DrawingmlCircleDraftV1,
    #[serde(rename = "drawingml-circle-anchor-focus-q96-v2-draft")]
    DrawingmlCircleAnchorFocusDraftV2,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxRadialLayoutRequest {
    pub profile: RadialLayoutProfile,
    pub fills: SourceFillQuery,
    pub options: RadialLayoutOptions,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxRadialLayoutResponse {
    Evaluated { plans: Vec<SourceRadialLayoutPlan> },
    Error { error: PptxFailure },
}
fn failure(error: SourceRadialLayoutError) -> PptxFailure {
    use SourceRadialLayoutError as E;
    if let E::At { target, error } = error {
        let mut result = failure(*error);
        result.message = format!("at {target:?}: {}", result.message);
        return result;
    }
    if let E::Source(error) = error {
        return pptx_failure(error);
    }
    let code = match &error {
        E::SourceConflict | E::Placement(SourcePlacementError::SourceConflict) => {
            PptxFailureCode::SourceConflict
        }
        E::Placement(SourcePlacementError::Cancelled)
        | E::Layout(
            RadialLayoutError::Cancelled
            | RadialLayoutError::Bounds(GeometryError::Cancelled)
            | RadialLayoutError::Path(NativePathError::Cancelled),
        ) => PptxFailureCode::Cancelled,
        E::Limit(_)
        | E::Placement(SourcePlacementError::Limit(_))
        | E::Layout(
            RadialLayoutError::Limit(_)
            | RadialLayoutError::Bounds(GeometryError::Limit(_))
            | RadialLayoutError::Path(NativePathError::Limit(_)),
        ) => PptxFailureCode::LimitExceeded,
        E::FillRequired(_) | E::GeometryRequired(_) | E::PlacementRequired(_) => {
            PptxFailureCode::MappingNotImplemented
        }
        _ => PptxFailureCode::InputInvalid,
    };
    PptxFailure {
        code,
        message: error.to_string(),
    }
}
pub fn layout_pptx_radial<R: ReaderAt>(
    request: &PptxRadialLayoutRequest,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    limits: RadialLayoutLimits,
    check: &dyn Fn() -> bool,
) -> PptxRadialLayoutResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .map_err(pptx_failure)
        .and_then(|index| {
            let basis = match request.profile {
                RadialLayoutProfile::DrawingmlCircleDraftV1 => {
                    CircleFocusBasis::CircumscribedSquare
                }
                RadialLayoutProfile::DrawingmlCircleAnchorFocusDraftV2 => {
                    CircleFocusBasis::AnchorRectangle
                }
            };
            layout_source_with_basis(
                &index,
                &request.fills,
                request.options,
                limits,
                basis,
                check,
            )
            .map_err(failure)
        });
    match result {
        Ok(plans) => PptxRadialLayoutResponse::Evaluated { plans },
        Err(error) => PptxRadialLayoutResponse::Error { error },
    }
}
pub fn layout_pptx_radial_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxRadialLayoutResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "radial layout request bytes".into(),
            },
        }
    } else {
        match from_json_str::<PptxRadialLayoutRequest>(input) {
            Ok(request) => layout_pptx_radial(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                RadialLayoutLimits::default(),
                &|| false,
            ),
            Err(error) => PptxRadialLayoutResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed radial layout response")
}
