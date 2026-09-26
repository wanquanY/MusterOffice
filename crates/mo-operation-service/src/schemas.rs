//! On-demand schemas from the same Rust types used by dispatch and codegen.
use crate::*;
use mo_common::Digest;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaId {
    HostRequest,
    HostResponse,
    OperationRequest,
    OperationJob,
    HostCapabilities,
    SchemaDocument,
    Document,
    UploadRequest,
    UploadInfo,
    AssetInfo,
}
impl SchemaId {
    pub const ALL: [Self; 10] = [
        Self::HostRequest,
        Self::HostResponse,
        Self::OperationRequest,
        Self::OperationJob,
        Self::HostCapabilities,
        Self::SchemaDocument,
        Self::Document,
        Self::UploadRequest,
        Self::UploadInfo,
        Self::AssetInfo,
    ];
    pub fn schema(self) -> schemars::Schema {
        let schema = match self {
            Self::HostRequest => schema_for!(HostRequest),
            Self::HostResponse => schema_for!(HostResponse),
            Self::OperationRequest => schema_for!(OperationRequest),
            Self::OperationJob => schema_for!(JobInfo),
            Self::HostCapabilities => schema_for!(HostCapabilities),
            Self::SchemaDocument => schema_for!(SchemaDocument),
            Self::Document => schema_for!(mo_presentation_model::Document),
            Self::UploadRequest => schema_for!(UploadRequest),
            Self::UploadInfo => schema_for!(UploadInfo),
            Self::AssetInfo => schema_for!(AssetInfo),
        };
        let name = serde_json::to_value(self).expect("static schema identifier");
        mo_common::runtime_schema(name.as_str().expect("string schema identifier"), schema)
    }
    pub fn document(self) -> Result<SchemaDocument, Failure> {
        let schema = serde_json::to_value(self.schema())
            .map_err(|_| Failure::new(FailureCode::InputInvalid, "schema encoding failed"))?;
        let digest = mo_common::digest("musteroffice.operation-schema/1", &(self, &schema))
            .map_err(|_| Failure::new(FailureCode::InputInvalid, "schema digest failed"))?;
        Ok(SchemaDocument {
            id: self,
            digest,
            schema,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemaDocument {
    pub id: SchemaId,
    /// Canonical domain-bound ID+schema digest, not a digest of formatted JSON.
    pub digest: Digest,
    pub schema: serde_json::Value,
}

/// Portable schema discovery for a browser host before it installs any owner
/// bridge. Input is a JSON SchemaId string; no document or authority is read.
pub fn operation_schema_json(input: &str) -> Result<String, Failure> {
    if input.len() > 128 {
        return Err(Failure::new(
            FailureCode::LimitExceeded,
            "schema identifier bytes",
        ));
    }
    let id: SchemaId = mo_common::from_json_str(input)
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "unknown schema identifier"))?;
    serde_json::to_string(&id.document()?)
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "schema document encoding failed"))
}
