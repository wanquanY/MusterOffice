//! One native source-to-page boundary shared by CLI and WASM hosts.
use super::pptx_source::{inline_limits, pptx_failure};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceIndex, SourceLimits, inspect_source},
};
pub use mo_presentation_compile::source_page::{
    SourcePageIssue, SourcePageLocation, SourcePagePlan, SourcePageProfile, SourcePageRasterInfo,
    SourcePageRequest,
};
use mo_presentation_compile::{
    native_paths::{NativePathError, NativePathIssue},
    source_page::{self, SourcePageError},
    source_placement::SourcePlacementError,
};
use mo_raster::{RasterBackend, RasterError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PptxPageFailureCode {
    InputInvalid,
    SourceConflict,
    PreservationConflict,
    MappingNotImplemented,
    ResourceRequired,
    LimitExceeded,
    Cancelled,
    ReadFailed,
    CoordinateRange,
    PrecisionExceeded,
    ComponentFailure,
    ComponentInvalid,
    HostFailure,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxPageFailure {
    pub code: PptxPageFailureCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourcePageLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<Box<SourcePageIssue>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPageCompileResponse {
    Compiled { plan: Box<SourcePagePlan> },
    Error { error: PptxPageFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPageRasterResponse {
    Rendered { info: Box<SourcePageRasterInfo> },
    Error { error: PptxPageFailure },
}
pub(crate) fn basic(code: PptxPageFailureCode, message: String) -> PptxPageFailure {
    PptxPageFailure {
        code,
        message,
        location: None,
        issue: None,
    }
}
fn source_failure(e: PptxError) -> PptxPageFailure {
    use super::PptxFailureCode as F;
    use PptxPageFailureCode::*;
    let e = pptx_failure(e);
    basic(
        match e.code {
            F::InputInvalid => InputInvalid,
            F::SourceConflict => SourceConflict,
            F::PreservationConflict => PreservationConflict,
            F::MappingNotImplemented => MappingNotImplemented,
            F::ResourceRequired => ResourceRequired,
            F::LimitExceeded => LimitExceeded,
            F::Cancelled => Cancelled,
            F::ReadFailed => ReadFailed,
        },
        e.message,
    )
}
pub(crate) fn failure(e: SourcePageError) -> PptxPageFailure {
    use PptxPageFailureCode::*;
    let message = e.to_string();
    let code = match e {
        SourcePageError::TextContextRequired {
            location,
            source_ordinal,
        } => {
            return PptxPageFailure {
                location: Some(location),
                issue: Some(Box::new(SourcePageIssue::Text { source_ordinal })),
                ..basic(ResourceRequired, message)
            };
        }
        SourcePageError::ImageResource(result) => match result.outcome {
            mo_pptx::source::images::SourceImageOutcome::NotImage {} => InputInvalid,
            mo_pptx::source::images::SourceImageOutcome::UnresolvedFill { .. } => {
                MappingNotImplemented
            }
            _ => ResourceRequired,
        },
        SourcePageError::ImageDecode(e) => match e {
            mo_image::ImageError::Invalid(_) | mo_image::ImageError::Component(1) => InputInvalid,
            mo_image::ImageError::Limit(_) | mo_image::ImageError::Component(3) => LimitExceeded,
            mo_image::ImageError::Component(5) => MappingNotImplemented,
            mo_image::ImageError::Cancelled => Cancelled,
            mo_image::ImageError::Component(_) => ComponentFailure,
            mo_image::ImageError::ComponentInvalid(_) => ComponentInvalid,
            mo_image::ImageError::Host(_) => HostFailure,
        },
        SourcePageError::ImageLayout(e) => match e {
            mo_presentation_compile::source_image_layout::ImageLayoutError::Invalid(_) => {
                InputInvalid
            }
            mo_presentation_compile::source_image_layout::ImageLayoutError::LexicalLimit => {
                LimitExceeded
            }
            mo_presentation_compile::source_image_layout::ImageLayoutError::Range => {
                CoordinateRange
            }
            mo_presentation_compile::source_image_layout::ImageLayoutError::Precision => {
                PrecisionExceeded
            }
            mo_presentation_compile::source_image_layout::ImageLayoutError::PhysicalSize(_) => {
                ResourceRequired
            }
            mo_presentation_compile::source_image_layout::ImageLayoutError::Cancelled => Cancelled,
        },
        SourcePageError::ImagePaint(e) => match e {
            mo_presentation_compile::source_image_paint::ImagePaintError::Invalid(_) => {
                InputInvalid
            }
            mo_presentation_compile::source_image_paint::ImagePaintError::OrientationRequired => {
                MappingNotImplemented
            }
            mo_presentation_compile::source_image_paint::ImagePaintError::Range => CoordinateRange,
            mo_presentation_compile::source_image_paint::ImagePaintError::Precision => {
                PrecisionExceeded
            }
            mo_presentation_compile::source_image_paint::ImagePaintError::Cancelled => Cancelled,
        },
        SourcePageError::AtObject { location, error } => {
            let mut result = failure(*error);
            result.location.get_or_insert(location);
            return result;
        }
        SourcePageError::Text(e) => return frame_failure(e),
        SourcePageError::TextPaint(e) => return paint_failure(e),
        SourcePageError::GlyphPaintConflict { .. } | SourcePageError::TextDecoration(_) => {
            MappingNotImplemented
        }
        SourcePageError::TextGeometry(e) => geometry_failure_code(e),
        SourcePageError::RadialLayout(e) => {
            use mo_presentation_compile::radial_layout::RadialLayoutError as E;
            match e {
                E::Invalid(_) => InputInvalid,
                E::Limit(_) => LimitExceeded,
                E::Range => CoordinateRange,
                E::Cancelled => Cancelled,
                E::Bounds(e) => geometry_failure_code(e),
                E::Path(e) => return failure(SourcePageError::Path(e)),
            }
        }
        SourcePageError::Source(e) => return source_failure(e),
        SourcePageError::Mapping { location, issue } => {
            return PptxPageFailure {
                location: Some(location),
                issue: Some(issue),
                ..basic(MappingNotImplemented, message)
            };
        }
        SourcePageError::SourceConflict
        | SourcePageError::Placement(SourcePlacementError::SourceConflict) => SourceConflict,
        SourcePageError::Invalid(_)
        | SourcePageError::Placement(SourcePlacementError::Invalid(_))
        | SourcePageError::Raster(RasterError::Invalid(_))
        | SourcePageError::Path(NativePathError::Options) => InputInvalid,
        SourcePageError::Placement(SourcePlacementError::Limit(_))
        | SourcePageError::Raster(RasterError::Limit(_))
        | SourcePageError::Path(NativePathError::Limit(_)) => LimitExceeded,
        SourcePageError::Placement(SourcePlacementError::Cancelled)
        | SourcePageError::Raster(RasterError::Cancelled)
        | SourcePageError::Path(NativePathError::Cancelled) => Cancelled,
        SourcePageError::Path(NativePathError::Geometry { issue, .. }) => match issue {
            NativePathIssue::NumericRange => CoordinateRange,
            NativePathIssue::PrecisionExceeded => PrecisionExceeded,
            _ => MappingNotImplemented,
        },
        SourcePageError::Raster(RasterError::Range) => CoordinateRange,
        SourcePageError::Raster(RasterError::Precision) => PrecisionExceeded,
        SourcePageError::Raster(RasterError::Component(_)) => ComponentFailure,
        SourcePageError::Raster(RasterError::ComponentInvalid(_)) => ComponentInvalid,
        SourcePageError::Raster(RasterError::Host(_)) => HostFailure,
    };
    basic(code, message)
}
fn paint_failure(e: mo_pptx::source::text::paint::TextPaintError) -> PptxPageFailure {
    use mo_pptx::source::text::paint::TextPaintError;
    match e {
        TextPaintError::AtRun { error, .. } => paint_failure(*error),
        TextPaintError::Source(e) => source_failure(e),
        e => basic(PptxPageFailureCode::MappingNotImplemented, e.to_string()),
    }
}
fn geometry_failure_code(e: mo_geometry::GeometryError) -> PptxPageFailureCode {
    use PptxPageFailureCode::*;
    use mo_geometry::GeometryError as E;
    match e {
        E::Numeric => CoordinateRange,
        E::Invalid(_) => InputInvalid,
        E::Limit(_) => LimitExceeded,
        E::Cancelled => Cancelled,
    }
}
fn frame_failure(e: mo_presentation_compile::source_frame::SourceFrameError) -> PptxPageFailure {
    use PptxPageFailureCode::*;
    use mo_presentation_compile::{
        CompileError as C, source_frame::SourceFrameError as E, source_text::SourceTextError as S,
    };
    let message = e.to_string();
    let code = match e {
        E::Source(e) | E::SourceText(S::Source(e)) => return source_failure(e),
        E::Table(e) => return table_frame_failure(e),
        E::SourceText(S::FontSelection(_)) => ResourceRequired,
        E::Text(e) | E::SourceText(S::Text(e)) => {
            use crate::text::ShapeFailureCode as F;
            match crate::text::shape_failure(e).code {
                F::InputInvalid | F::FontInvalid => InputInvalid,
                F::Unsupported => MappingNotImplemented,
                F::ResourceConflict => SourceConflict,
                F::ResourceRequired => ResourceRequired,
                F::LimitExceeded => LimitExceeded,
                F::Cancelled => Cancelled,
                F::ComponentInvalid => ComponentInvalid,
                F::ComponentFailure => ComponentFailure,
                F::HostFailure => HostFailure,
            }
        }
        E::Mapping(_) => MappingNotImplemented,
        E::Geometry(e) => geometry_failure_code(e),
        E::Coordinate(C::Range) => CoordinateRange,
        E::Coordinate(C::Cancelled) | E::Cancelled | E::SourceText(S::Cancelled) => Cancelled,
        E::Coordinate(C::Invalid(_) | C::Document(_)) | E::SourceText(S::Invalid(_)) => {
            InputInvalid
        }
        E::Limit(_)
        | E::SourceText(S::Limit(_))
        | E::Coordinate(mo_presentation_compile::CompileError::Limit(_)) => LimitExceeded,
    };
    basic(code, message)
}
fn table_frame_failure(
    e: mo_presentation_compile::source_table::TableGeometryError,
) -> PptxPageFailure {
    use PptxPageFailureCode::*;
    use mo_pptx::source::table::grid::NativeTableGridError as G;
    use mo_presentation_compile::{CompileError as C, source_table::TableGeometryError as E};
    let message = e.to_string();
    let code = match e {
        E::Source(e) => return source_failure(e),
        E::Coordinate(C::Range) => CoordinateRange,
        E::Cancelled | E::Grid(G::Cancelled) | E::Coordinate(C::Cancelled) => Cancelled,
        E::Limit(_) | E::Grid(G::Limit) | E::Coordinate(C::Limit(_)) => LimitExceeded,
        E::NegativeDimension { .. }
        | E::Grid(G::Invalid(_))
        | E::Coordinate(C::Invalid(_) | C::Document(_)) => InputInvalid,
    };
    basic(code, message)
}
pub(crate) fn inspect<R: ReaderAt>(
    reader: R,
    length: u64,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceIndex, PptxPageFailure> {
    Package::open(reader, length, limits.package, check)
        .map_err(PptxError::from)
        .and_then(|p| inspect_source(&p, limits, check))
        .map_err(source_failure)
}
pub fn compile_pptx_page<R: ReaderAt>(
    request: &SourcePageRequest,
    reader: R,
    length: u64,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> PptxPageCompileResponse {
    match inspect(reader, length, limits, check)
        .and_then(|i| source_page::compile(&i, request, check).map_err(failure))
    {
        Ok(plan) => PptxPageCompileResponse::Compiled {
            plan: Box::new(plan),
        },
        Err(error) => PptxPageCompileResponse::Error { error },
    }
}
pub fn render_pptx_page<R: ReaderAt>(
    request: &SourcePageRequest,
    reader: R,
    length: u64,
    limits: SourceLimits,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (PptxPageRasterResponse, Vec<u8>) {
    match inspect(reader, length, limits, check)
        .and_then(|i| source_page::render(&i, request, backend, check).map_err(failure))
    {
        Ok(image) => (
            PptxPageRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (PptxPageRasterResponse::Error { error }, vec![]),
    }
}
fn parse(input: &str) -> Result<SourcePageRequest, PptxPageFailure> {
    if input.len() > super::MAX_REQUEST_BYTES {
        return Err(basic(
            PptxPageFailureCode::LimitExceeded,
            "source page request bytes".into(),
        ));
    }
    from_json_str(input).map_err(|e| basic(PptxPageFailureCode::InputInvalid, e.to_string()))
}
pub fn compile_pptx_page_json(input: &str, source: &[u8]) -> String {
    let response = match parse(input) {
        Ok(q) => compile_pptx_page(&q, source, source.len() as u64, inline_limits(), &|| false),
        Err(error) => PptxPageCompileResponse::Error { error },
    };
    serde_json::to_string(&response).expect("typed source page response")
}
pub fn render_pptx_page_json(
    input: &str,
    source: &[u8],
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let (response, pixels) = match parse(input) {
        Ok(q) => render_pptx_page(
            &q,
            source,
            source.len() as u64,
            inline_limits(),
            backend,
            check,
        ),
        Err(error) => (PptxPageRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("typed source page raster response"),
        pixels,
    )
}

#[cfg(test)]
mod table_frame_failure_tests {
    use super::*;
    use mo_pptx::source::table::grid::{NativeTableGridError as G, NativeTableGridIssue};
    use mo_presentation_compile::{
        CompileError as C, source_frame::SourceFrameError, source_table::TableGeometryError as E,
    };
    #[test]
    fn nested_table_errors_keep_cancellation_limit_conflict_and_input_categories() {
        for (e, expected) in [
            (E::Cancelled, "CANCELLED"),
            (E::Grid(G::Cancelled), "CANCELLED"),
            (E::Coordinate(C::Cancelled), "CANCELLED"),
            (E::Limit("table"), "LIMIT_EXCEEDED"),
            (E::Grid(G::Limit), "LIMIT_EXCEEDED"),
            (E::Coordinate(C::Limit("number")), "LIMIT_EXCEEDED"),
            (E::Coordinate(C::Range), "COORDINATE_RANGE"),
            (E::Coordinate(C::Invalid("dimension")), "INPUT_INVALID"),
            (
                E::Grid(G::Invalid(NativeTableGridIssue::EmptyGrid)),
                "INPUT_INVALID",
            ),
            (E::NegativeDimension { source_ordinal: 12 }, "INPUT_INVALID"),
            (
                E::Source(PptxError::SourceConflict("stale source".into())),
                "SOURCE_CONFLICT",
            ),
        ] {
            let result = failure(SourcePageError::Text(SourceFrameError::Table(e)));
            assert_eq!(serde_json::to_value(result.code).unwrap(), expected);
        }
    }
}
