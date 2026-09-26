//! Standard MCP projections of the same authorized, durable operation owner.
use crate::{bridge::HostBridge, catalog, resources::ResourceSpace};
use mo_operation_service::{Failure, FailureCode, HostResponse};
use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};
use std::{borrow::Cow, sync::Arc};

pub struct OfficeServer {
    host: Arc<HostBridge>,
    tools: Vec<Tool>,
    resources: ResourceSpace,
}
impl OfficeServer {
    /// Bootstrap outside the async service: opening SQLite is blocking I/O.
    pub fn new(host: Arc<HostBridge>, resources: ResourceSpace) -> Result<Self, Failure> {
        let tools = catalog::tools(&host.capabilities()?);
        Ok(Self {
            host,
            tools,
            resources,
        })
    }
}
fn no_cursor(request: Option<PaginatedRequestParams>) -> Result<(), ErrorData> {
    if request.is_some_and(|p| p.cursor.is_some()) {
        Err(ErrorData::invalid_params("No page cursor", None))
    } else {
        Ok(())
    }
}
fn supports_cache(context: &RequestContext<RoleServer>) -> bool {
    context
        .protocol_version()
        .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28)
}
impl ServerHandler for OfficeServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().build())
            .with_server_info(Implementation::new("MusterOffice", env!("CARGO_PKG_VERSION")))
            .with_instructions("Development adapter for the current draft operation contract. Read mo_capabilities and musteroffice://adapter/limits. Accepted jobs are durable; poll mo_jobs_get. Request cancellation does not cancel a business job; use mo_jobs_cancel. Full presentation and Office/WPS acceptance remain incomplete.")
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![
            ProtocolVersion::V_2025_11_25,
            ProtocolVersion::V_2026_07_28,
        ])
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.iter().find(|t| t.name == name).cloned()
    }
    async fn list_prompts(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        // This adapter does not advertise prompts. Do not inherit the SDK's
        // empty successful list (which is another cacheable result in 2026).
        Err(ErrorData::method_not_found::<ListPromptsRequestMethod>())
    }
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        no_cursor(request)?;
        Ok(ListToolsResult {
            tools: self.tools.clone(),
            ttl_ms: supports_cache(&context).then_some(0),
            cache_scope: supports_cache(&context).then_some(CacheScope::Private),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if !self.tools.iter().any(|tool| tool.name == request.name) {
            return Err(ErrorData::invalid_params(
                "Unknown or unavailable tool",
                None,
            ));
        }
        let binding = catalog::BINDINGS
            .iter()
            .find(|b| b.name == request.name)
            .expect("catalog binding");
        let response = match binding.decode(request.arguments.unwrap_or_default()) {
            Err(message) => HostResponse::Failed {
                error: Failure::new(FailureCode::InputInvalid, message),
                job: None,
            },
            Ok(request) => {
                // Keep the full context (and transport admission guard) alive
                // until this handler returns. Dropping the wait does not drop
                // the blocking host execution permit or mutate business state.
                let response = tokio::select! {
                    response = self.host.dispatch(request) => response,
                    _ = context.ct.cancelled() => Err(Failure::new(FailureCode::Cancelled, "protocol wait cancelled; query the original business request/job")),
                };
                response.unwrap_or_else(|error| HostResponse::Failed { error, job: None })
            }
        };
        let links = self.resources.links(&response);
        let is_error = matches!(response, HostResponse::Failed { .. });
        let mut result =
            CallToolResult::structured(serde_json::to_value(response).expect("host result JSON"));
        result.is_error = Some(is_error);
        result.content.extend(links);
        drop(context);
        Ok(result.into())
    }
    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        no_cursor(request)?;
        Ok(ListResourcesResult {
            resources: self.resources.list(),
            ttl_ms: supports_cache(&context).then_some(0),
            cache_scope: supports_cache(&context).then_some(CacheScope::Private),
            ..Default::default()
        })
    }
    async fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        no_cursor(request)?;
        Ok(ListResourceTemplatesResult {
            resource_templates: self.resources.templates(),
            ttl_ms: supports_cache(&context).then_some(0),
            cache_scope: supports_cache(&context).then_some(CacheScope::Private),
            ..Default::default()
        })
    }
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let content = tokio::select! {
            result = self.resources.read(&request.uri, &self.host) => result?,
            _ = context.ct.cancelled() => return Err(ErrorData::internal_error("protocol resource wait cancelled", None)),
        };
        let mut result = ReadResourceResult::new(vec![content]);
        if supports_cache(&context) {
            result = result.with_ttl_ms(0).with_cache_scope(CacheScope::Private);
        }
        drop(context);
        Ok(result.into())
    }
}
