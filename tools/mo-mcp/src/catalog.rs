//! Thin wire projections of the actual operation contract, never parallel IDL.
use mo_operation_service::{HostCapabilities, HostRequest, SchemaId, ServiceOperation};
use rmcp::model::{Tool, ToolAnnotations};
use serde_json::{Map, Value};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Copy)]
pub struct Binding {
    pub name: &'static str,
    pub operation: ServiceOperation,
    pub host_tag: &'static str,
    pub action: Option<&'static str>,
    description: &'static str,
}
impl Binding {
    pub fn decode(&self, mut arguments: Map<String, Value>) -> Result<HostRequest, &'static str> {
        if arguments.contains_key("operation") {
            return Err("operation is fixed by the tool name");
        }
        arguments.insert("operation".into(), self.host_tag.into());
        let request: HostRequest = serde_json::from_value(Value::Object(arguments))
            .map_err(|_| "invalid operation arguments")?;
        if request.service_operation() != self.operation {
            return Err("action does not match tool name");
        }
        Ok(request)
    }
    fn schema(&self) -> Map<String, Value> {
        let source = serde_json::to_value(SchemaId::HostRequest.schema()).expect("shared schema");
        let mut projected = source["oneOf"]
            .as_array()
            .expect("host union")
            .iter()
            .find(|v| v["properties"]["operation"]["const"] == self.host_tag)
            .expect("binding tag")
            .clone();
        projected["properties"]
            .as_object_mut()
            .unwrap()
            .remove("operation");
        projected["required"]
            .as_array_mut()
            .unwrap()
            .retain(|v| v != "operation");
        let mut defs = source["$defs"].as_object().unwrap().clone();
        if let Some(action) = self.action {
            defs["DocumentAction"]["oneOf"]
                .as_array_mut()
                .unwrap()
                .retain(|v| v["properties"]["kind"]["const"] == action);
            assert_eq!(defs["DocumentAction"]["oneOf"].as_array().unwrap().len(), 1);
        }
        let mut reachable = BTreeSet::new();
        references(&projected, &mut reachable);
        loop {
            let before = reachable.len();
            for name in reachable.clone() {
                references(&defs[&name], &mut reachable);
            }
            if reachable.len() == before {
                break;
            }
        }
        defs.retain(|name, _| reachable.contains(name));
        projected["$defs"] = Value::Object(defs);
        projected["$schema"] = source["$schema"].clone();
        projected.as_object().unwrap().clone()
    }
    pub fn tool(&self, output: Arc<Map<String, Value>>) -> Tool {
        let read_only = matches!(
            self.operation,
            ServiceOperation::Capabilities
                | ServiceOperation::Schema
                | ServiceOperation::ReadDocument
                | ServiceOperation::ReadAsset
                | ServiceOperation::GetUpload
                | ServiceOperation::GetJob
        );
        Tool::new(self.name, self.description, self.schema())
            .with_raw_output_schema(output)
            .with_annotations(
                ToolAnnotations::new()
                    .read_only(read_only)
                    .idempotent(true)
                    .destructive(matches!(
                        self.operation,
                        ServiceOperation::CancelJob | ServiceOperation::CancelUpload
                    ))
                    .open_world(false),
            )
    }
}
fn references(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(name) = map
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix("#/$defs/"))
            {
                names.insert(name.into());
            }
            for v in map.values() {
                references(v, names);
            }
        }
        Value::Array(values) => {
            for v in values {
                references(v, names);
            }
        }
        _ => (),
    }
}

pub const BINDINGS: &[Binding] = &[
    Binding {
        name: "mo_capabilities",
        operation: ServiceOperation::Capabilities,
        host_tag: "capabilities",
        action: None,
        description: "Read actual host capabilities and declared limits. Full presentation acceptance is reported separately.",
    },
    Binding {
        name: "mo_schemas_get",
        operation: ServiceOperation::Schema,
        host_tag: "getSchema",
        action: None,
        description: "Read a canonical schema from the same kernel types used by every operation.",
    },
    Binding {
        name: "mo_presentations_create",
        operation: ServiceOperation::Create,
        host_tag: "submit",
        action: Some("create"),
        description: "Create an editable presentation from a typed document. Reuse requestId only for identical logical arguments. Accepted means a durable job, not a finished document.",
    },
    Binding {
        name: "mo_presentations_apply",
        operation: ServiceOperation::Apply,
        host_tag: "submit",
        action: Some("apply"),
        description: "Apply an atomic edit to the expected revision. Use one transaction for related changes; a conflict never publishes a partial edit.",
    },
    Binding {
        name: "mo_presentations_export",
        operation: ServiceOperation::Export,
        host_tag: "submit",
        action: Some("export"),
        description: "Export a fixed revision to actual editable PPTX, previews and quality evidence. Assets become readable only after the complete candidate commits.",
    },
    Binding {
        name: "mo_presentations_read",
        operation: ServiceOperation::ReadDocument,
        host_tag: "readDocument",
        action: None,
        description: "Read an authorized immutable revision or the current document head.",
    },
    Binding {
        name: "mo_assets_prepare",
        operation: ServiceOperation::BeginUpload,
        host_tag: "beginUpload",
        action: None,
        description: "Reserve an upload by length, media type and SHA256. Binary bytes use the native host data channel, not model arguments.",
    },
    Binding {
        name: "mo_assets_get_upload",
        operation: ServiceOperation::GetUpload,
        host_tag: "getUpload",
        action: None,
        description: "Read an authorized upload state and its verified asset after sealing.",
    },
    Binding {
        name: "mo_assets_seal",
        operation: ServiceOperation::SealUpload,
        host_tag: "sealUpload",
        action: None,
        description: "Verify actual uploaded bytes and seal them as an immutable authorized asset.",
    },
    Binding {
        name: "mo_assets_cancel_upload",
        operation: ServiceOperation::CancelUpload,
        host_tag: "cancelUpload",
        action: None,
        description: "Cancel a temporary upload according to its durable state; published assets are preserved.",
    },
    Binding {
        name: "mo_assets_read",
        operation: ServiceOperation::ReadAsset,
        host_tag: "readAsset",
        action: None,
        description: "Read an authorized asset descriptor. Read small bytes via the returned resource URI; large bytes use the native host data channel.",
    },
    Binding {
        name: "mo_jobs_get",
        operation: ServiceOperation::GetJob,
        host_tag: "getJob",
        action: None,
        description: "Read durable job status and committed results. A disconnected request does not cancel a job.",
    },
    Binding {
        name: "mo_jobs_cancel",
        operation: ServiceOperation::CancelJob,
        host_tag: "cancelJob",
        action: None,
        description: "Request durable business cancellation. A successful prior commit remains successful and readable.",
    },
];

pub fn tools(capabilities: &HostCapabilities) -> Vec<Tool> {
    let output = Arc::new(
        serde_json::to_value(SchemaId::HostResponse.schema())
            .unwrap()
            .as_object()
            .unwrap()
            .clone(),
    );
    let mut result: Vec<_> = BINDINGS
        .iter()
        .filter(|b| {
            capabilities
                .operations
                .iter()
                .any(|o| o.operation == b.operation)
        })
        .map(|b| b.tool(output.clone()))
        .collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}
