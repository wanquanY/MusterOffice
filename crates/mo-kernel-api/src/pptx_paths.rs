//! Host-independent native path compilation from a digest-bound source package.
use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::{Digest, from_json_str};
use mo_geometry::Fixed;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{
        SourceLimits, SourceResolvedValue,
        geometry::evaluate::{
            self, GeometryLimits, GeometryOutcome, GeometryProfile, GeometryUnresolved,
            SourceGeometryQuery,
        },
        inspect_source,
    },
};
pub use mo_presentation_compile::native_paths::{
    CompiledNativePath, NativePathIssue, NativePathLimits, NativePathOptions, NativePathProfile,
};
use mo_presentation_compile::native_paths::{NativePathCompiler, NativePathError};
use mo_presentation_model::Size;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceNativePathsQuery {
    pub geometry: SourceGeometryQuery,
    pub options: NativePathOptions,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum NativeObjectPathsOutcome {
    Compiled {
        extent: SourceResolvedValue<Size>,
        paths: Vec<CompiledNativePath>,
    },
    UnresolvedGeometry {
        reason: GeometryUnresolved,
    },
    UnresolvedPath {
        origin: mo_pptx::source::geometry::evaluate::GeometryOrigin,
        issue: NativePathIssue,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeObjectPaths {
    pub native_id: u32,
    pub outcome: NativeObjectPathsOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceNativePaths {
    pub source_sha256: Digest,
    pub surface: String,
    pub geometry_profile: GeometryProfile,
    pub profile: NativePathProfile,
    pub coordinate_tolerance: Fixed,
    pub objects: Vec<NativeObjectPaths>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPathsResponse {
    Compiled { paths: SourceNativePaths },
    Error { error: PptxFailure },
}
fn failure(error: NativePathError) -> PptxFailure {
    let code = match error {
        NativePathError::Cancelled => PptxFailureCode::Cancelled,
        NativePathError::Limit(_) => PptxFailureCode::LimitExceeded,
        _ => PptxFailureCode::InputInvalid,
    };
    PptxFailure {
        code,
        message: error.to_string(),
    }
}
pub fn compile_pptx_paths<R: ReaderAt>(
    request: &SourceNativePathsQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    geometry_limits: GeometryLimits,
    path_limits: NativePathLimits,
    check: &dyn Fn() -> bool,
) -> PptxPathsResponse {
    let run = || -> Result<SourceNativePaths, PptxFailure> {
        let mut compiler = NativePathCompiler::new(request.options.clone(), path_limits, check)
            .map_err(failure)?;
        let index = Package::open(reader, length, source_limits.package, check)
            .map_err(PptxError::from)
            .and_then(|package| inspect_source(&package, source_limits, check))
            .map_err(pptx_failure)?;
        let values = evaluate::query(&index, &request.geometry, geometry_limits, check)
            .map_err(pptx_failure)?;
        let mut objects = Vec::new();
        for result in values.objects {
            let outcome = match result.outcome {
                GeometryOutcome::Unresolved { reason } => {
                    NativeObjectPathsOutcome::UnresolvedGeometry { reason }
                }
                GeometryOutcome::Resolved { geometry } => match compiler.compile(&geometry) {
                    Ok(paths) => NativeObjectPathsOutcome::Compiled {
                        extent: geometry.extent,
                        paths,
                    },
                    Err(NativePathError::Geometry { origin, issue }) => {
                        NativeObjectPathsOutcome::UnresolvedPath { origin, issue }
                    }
                    Err(error) => return Err(failure(error)),
                },
            };
            if check() {
                return Err(failure(NativePathError::Cancelled));
            }
            objects.push(NativeObjectPaths {
                native_id: result.native_id,
                outcome,
            });
        }
        Ok(SourceNativePaths {
            source_sha256: values.source_sha256,
            surface: values.surface,
            geometry_profile: values.profile,
            profile: request.options.profile,
            coordinate_tolerance: request.options.coordinate_tolerance,
            objects,
        })
    };
    match run() {
        Ok(paths) => PptxPathsResponse::Compiled { paths },
        Err(error) => PptxPathsResponse::Error { error },
    }
}
pub fn compile_pptx_paths_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxPathsResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "native path request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceNativePathsQuery>(input) {
            Ok(request) => compile_pptx_paths(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                GeometryLimits::default(),
                NativePathLimits::default(),
                &|| false,
            ),
            Err(error) => PptxPathsResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed path response is serializable")
}
