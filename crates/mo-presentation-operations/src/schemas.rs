//! Computation-only schema discovery. Legacy host schemas have a separate API.
use crate::{
    ComputationReceipt, ExportReceipt, Failure, FailureCode, Invocation, MAX_OPERATION_BYTES,
    MutationReceipt, OperationRequest,
};
use mo_common::Digest;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaId {
    ComputationRequest,
    AuthoringAction,
    ComputationFailure,
    ComputationMutationReceipt,
    ComputationExportReceipt,
    ComputationInvocation,
    ComputationReceipt,
    TemplateDefinition,
    TemplateRequest,
    TemplateResponse,
}
impl SchemaId {
    pub const ALL: [Self; 10] = [
        Self::ComputationRequest,
        Self::AuthoringAction,
        Self::ComputationFailure,
        Self::ComputationMutationReceipt,
        Self::ComputationExportReceipt,
        Self::ComputationInvocation,
        Self::ComputationReceipt,
        Self::TemplateDefinition,
        Self::TemplateRequest,
        Self::TemplateResponse,
    ];
    pub fn schema(self) -> schemars::Schema {
        let schema = match self {
            Self::ComputationRequest => schema_for!(OperationRequest),
            Self::AuthoringAction => schema_for!(crate::compose::authoring::AuthoringAction),
            Self::ComputationFailure => schema_for!(Failure),
            Self::ComputationMutationReceipt => schema_for!(MutationReceipt),
            Self::ComputationExportReceipt => schema_for!(ExportReceipt),
            Self::ComputationInvocation => schema_for!(Invocation),
            Self::ComputationReceipt => schema_for!(ComputationReceipt),
            Self::TemplateDefinition => schema_for!(mo_presentation_template::TemplateDefinition),
            Self::TemplateRequest => schema_for!(mo_presentation_template::TemplateRequest),
            Self::TemplateResponse => schema_for!(mo_presentation_template::TemplateResponse),
        };
        let name = serde_json::to_value(self).expect("static schema identifier");
        mo_common::runtime_schema(name.as_str().expect("string schema identifier"), schema)
    }
    pub fn document(self) -> Result<SchemaDocument, Failure> {
        let schema = serde_json::to_value(self.schema())
            .map_err(|_| Failure::new(FailureCode::InputInvalid, "schema encoding failed"))?;
        let digest = mo_common::digest("musteroffice.computation-schema/1", &(self, &schema))
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
    pub digest: Digest,
    pub schema: serde_json::Value,
}
pub fn computation_schema_json(input: &str) -> Result<String, Failure> {
    if input.len() > 128 {
        return Err(Failure::new(
            FailureCode::LimitExceeded,
            "schema identifier bytes",
        ));
    }
    let id: SchemaId = mo_common::from_json_str(input)
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "unknown computation schema"))?;
    serde_json::to_string(&id.document()?)
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "schema document encoding failed"))
}
pub fn decode_request(input: &str) -> Result<OperationRequest, Failure> {
    if input.len() > MAX_OPERATION_BYTES {
        return Err(Failure::new(
            FailureCode::LimitExceeded,
            "computation request bytes",
        ));
    }
    let request: OperationRequest = mo_common::from_json_str(input)
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "invalid computation request"))?;
    request.validate_profile()?;
    Ok(request)
}
