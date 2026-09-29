//! Stateless, bounded calculation envelope shared by native and WASM callers.
//! Source revision/material authorization and final publication remain external.
use crate::*;
use mo_presentation_edit::SnapshotRecord;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum TemplateComputationVersion {
    #[serde(rename = "musteroffice.template-computation/1-draft")]
    V1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateRequest {
    pub version: TemplateComputationVersion,
    pub source: SnapshotRecord,
    pub definition: TemplateDefinition,
    pub action: TemplateAction,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TemplateAction {
    Describe {},
    Instantiate { request: InstantiateRequest },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum TemplateResponse {
    Described {
        description: Box<TemplateDescription>,
    },
    Instantiated {
        instance: Box<TemplateInstance>,
    },
    Error {
        error: TemplateDiagnostic,
    },
}
impl TemplateResponse {
    fn error(error: TemplateError) -> Self {
        Self::Error {
            error: error.diagnostic(),
        }
    }
}
pub fn compute_template(
    request: TemplateRequest,
    limits: TemplateLimits,
    check: &dyn Fn() -> bool,
) -> TemplateResponse {
    fn run(
        request: TemplateRequest,
        limits: TemplateLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TemplateResponse, TemplateError> {
        cancelled(check)?;
        check_size(&request, limits.max_bytes, check)?;
        let source = Snapshot::restore(request.source, limits.document)?;
        let template = Template::new(source, request.definition, limits, check)?;
        let result = match request.action {
            TemplateAction::Describe {} => TemplateResponse::Described {
                description: Box::new(template.describe(check)?),
            },
            TemplateAction::Instantiate { request } => TemplateResponse::Instantiated {
                instance: Box::new(template.instantiate(&request, check)?),
            },
        };
        check_size(&result, limits.max_bytes, check)?;
        Ok(result)
    }
    run(request, limits, check).unwrap_or_else(TemplateResponse::error)
}
pub fn compute_template_json(
    input: &str,
    limits: TemplateLimits,
    check: &dyn Fn() -> bool,
) -> String {
    let response = if check() {
        TemplateResponse::error(TemplateError::Cancelled)
    } else if input.len() > limits.max_bytes {
        TemplateResponse::error(TemplateError::LimitExceeded("request bytes"))
    } else {
        match mo_common::from_json_str(input) {
            Ok(request) => compute_template(request, limits, check),
            Err(error) => {
                TemplateResponse::error(TemplateError::InvalidTemplate(error.to_string()))
            }
        }
    };
    serde_json::to_string(&response).expect("typed template response")
}
