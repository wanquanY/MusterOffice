//! SDK selection probe only. No claim of a complete MCP adapter or task host.
use futures::StreamExt;
use mo_operation_service::SchemaId;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext,
    transport::async_rw::JsonRpcMessageCodec,
};
use std::{
    borrow::Cow,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Query {
    id: SchemaId,
}
struct Probe;
impl Probe {
    fn tool() -> Tool {
        let input = serde_json::to_value(schemars::schema_for!(Query)).unwrap();
        let output = serde_json::to_value(SchemaId::SchemaDocument.schema()).unwrap();
        Tool::new(
            "mo_probe_schema",
            "Read a real shared kernel schema; selection probe only.",
            input.as_object().unwrap().clone(),
        )
        .with_raw_output_schema(Arc::new(output.as_object().unwrap().clone()))
    }
}
impl ServerHandler for Probe {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![
            ProtocolVersion::V_2025_11_25,
            ProtocolVersion::V_2026_07_28,
        ])
    }
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        if request.is_some_and(|p| p.cursor.is_some()) {
            return Err(ErrorData::invalid_params("No page cursor", None));
        }
        Ok(ListToolsResult {
            tools: vec![Self::tool()],
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        (name == "mo_probe_schema").then(Self::tool)
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if request.name != "mo_probe_schema" {
            return Err(ErrorData::method_not_found::<CallToolRequestMethod>());
        }
        let query: Query = serde_json::from_value(serde_json::Value::Object(
            request.arguments.unwrap_or_default(),
        ))
        .map_err(|_| ErrorData::invalid_params("Invalid schema query", None))?;
        let document = query
            .id
            .document()
            .map_err(|_| ErrorData::internal_error("Schema computation failed", None))?;
        Ok(CallToolResult::structured(serde_json::to_value(document).unwrap()).into())
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Both message parsing and line buffering have an explicit byte boundary.
    // A bad frame ends this probe connection; it is never skipped into success.
    let failed = Arc::new(AtomicBool::new(false));
    let on_error = failed.clone();
    let reader = FramedRead::new(
        tokio::io::stdin(),
        JsonRpcMessageCodec::<ClientJsonRpcMessage>::new_with_max_length(4096),
    )
    .take_while(move |r| {
        if r.is_err() {
            on_error.store(true, Ordering::Relaxed);
        }
        std::future::ready(r.is_ok())
    })
    .map(Result::unwrap);
    let writer = FramedWrite::new(
        tokio::io::stdout(),
        JsonRpcMessageCodec::<ServerJsonRpcMessage>::default(),
    );
    let service = Probe.serve((writer, reader)).await?;
    service.waiting().await?;
    if failed.load(Ordering::Relaxed) {
        return Err("invalid or over-budget MCP frame".into());
    }
    Ok(())
}
