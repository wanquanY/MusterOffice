//! Pure import admission for binary transports. Host authority and publication
//! stay outside this call; its snapshot uses the normal document revision type.
use crate::{
    MAX_INLINE_RESOURCE_BYTES, MAX_REQUEST_BYTES, PptxFailure,
    pptx_source::{inline_limits, pptx_failure},
};
use mo_common::{Digest, DocumentId, ResourceId};
use mo_pptx::PptxError;
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxImportRequest {
    pub document_id: DocumentId,
    pub resource_id: ResourceId,
    pub expected_source_sha256: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxImportResponse {
    Imported { snapshot: Box<SnapshotRecord> },
    Error { error: PptxFailure },
}
pub fn import_pptx_document(
    request: &PptxImportRequest,
    source: &[u8],
    check: &dyn Fn() -> bool,
) -> PptxImportResponse {
    let result = (|| {
        if source.len() > MAX_INLINE_RESOURCE_BYTES {
            return Err(PptxError::Limit("import source bytes"));
        }
        let limits = inline_limits();
        let package = mo_opc::Package::open(source, source.len() as u64, limits.package, check)?;
        if package.sha256() != &request.expected_source_sha256 {
            return Err(PptxError::SourceConflict("import source identity".into()));
        }
        let document = mo_pptx::source::document::import_document(
            &package,
            request.document_id.clone(),
            request.resource_id.clone(),
            limits,
            check,
        )?;
        Snapshot::new(document, Default::default())
            .map(Snapshot::into_record)
            .map_err(|e| PptxError::SourceConflict(e.to_string()))
    })();
    match result {
        Ok(snapshot) => PptxImportResponse::Imported {
            snapshot: Box::new(snapshot),
        },
        Err(error) => PptxImportResponse::Error {
            error: pptx_failure(error),
        },
    }
}
pub fn import_pptx_document_json(request: &str, source: &[u8]) -> String {
    let response = if request.len() > MAX_REQUEST_BYTES {
        PptxImportResponse::Error {
            error: pptx_failure(PptxError::Limit("import request bytes")),
        }
    } else {
        match mo_common::from_json_str(request) {
            Ok(request) => import_pptx_document(&request, source, &|| false),
            Err(error) => PptxImportResponse::Error {
                error: pptx_failure(PptxError::Value {
                    path: "request".into(),
                    message: error.to_string(),
                }),
            },
        }
    };
    serde_json::to_string(&response).expect("typed import response")
}
