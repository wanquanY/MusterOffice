use super::{Bridge, FileArguments, Response, resources};
use mo_embedded_sdk::operation::{Failure, FailureCode, SchemaId};
use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use std::{borrow::Cow, sync::Arc};

#[derive(Clone)]
pub struct ComputeServer {
    bridge: Arc<Bridge>,
    tools: Arc<Vec<Tool>>,
    http: bool,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SchemaArgument {
    id: SchemaId,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Empty {}

fn schema<T: JsonSchema>() -> serde_json::Map<String, serde_json::Value> {
    serde_json::to_value(schema_for!(T))
        .expect("adapter schema")
        .as_object()
        .expect("object schema")
        .clone()
}
impl ComputeServer {
    #[cfg(feature = "http")]
    pub(crate) fn http(bridge: Arc<Bridge>) -> Self {
        let mut server = Self::new(bridge);
        server.http = true;
        server
    }
    pub fn new(bridge: Arc<Bridge>) -> Self {
        let mut tools=vec![
            Tool::new("mo_capabilities","Read computation, file-channel and current implementation limits. No account or persistent job service.",schema::<Empty>()).with_annotations(ToolAnnotations::new().read_only(true).idempotent(true).open_world(false)),
            Tool::new("mo_schema","Read the shared SDK computation schema; use it to author the caller-owned invocation file.",schema::<SchemaArgument>()).with_annotations(ToolAnnotations::new().read_only(true).idempotent(true).open_world(false)),
            Tool::new("mo_presentations_compute","Create, import, describe or instantiate a pinned template, atomically edit or export using an invocation JSON and input manifest in the configured input directory. Template description validates the invocation snapshot and parameter targets and returns the template digest and real examples. Instantiation uses that immutable source and returns a new document. Supply portable leaf names and a NEW output directory name. Files belong to the caller; no database, job or publication is created. Large document bytes use the caller's file channel, not tool JSON.",schema::<FileArguments>()).with_raw_output_schema(Arc::new(schema::<Response>())).with_annotations(ToolAnnotations::new().read_only(false).destructive(false).idempotent(false).open_world(false)),
        ];
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            bridge,
            tools: Arc::new(tools),
            http: false,
        }
    }
}
fn no_cursor(request: Option<PaginatedRequestParams>) -> Result<(), ErrorData> {
    if request.is_some_and(|p| p.cursor.is_some()) {
        Err(ErrorData::invalid_params("No page cursor", None))
    } else {
        Ok(())
    }
}
fn cached(context: &RequestContext<RoleServer>) -> bool {
    context
        .protocol_version()
        .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28)
}
fn failed(error: Failure) -> CallToolResponse {
    let mut result = CallToolResult::structured(
        serde_json::to_value(Response::Failed { error }).expect("failure JSON"),
    );
    result.is_error = Some(true);
    result.into()
}
impl ServerHandler for ComputeServer {
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        if self.http {
            // Stateless HTTP has no initialize handshake. The SDK's legacy
            // negotiation fallback must not expand this endpoint's scope.
            return Err(
                if request.protocol_version == ProtocolVersion::V_2026_07_28 {
                    ErrorData::method_not_found::<InitializeResultMethod>()
                } else {
                    ErrorData::unsupported_protocol_version(
                        request.protocol_version,
                        &self.supported_protocol_versions(),
                    )
                },
            );
        }
        context.peer.set_peer_info(request.clone());
        self.negotiate_initialize(&request)
    }
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().build())
            .with_server_info(Implementation::new("MusterOffice",env!("CARGO_PKG_VERSION")))
            .with_instructions("Computation-only caller-file adapter. Read mo_capabilities and mo_schema. The caller supplies and owns input/output files. Protocol cancellation cancels actual computation; no durable jobs or Tasks are advertised. Results are calculations, not product publication. Current full-presentation and Office/WPS acceptance remain incomplete.")
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        if self.http {
            Cow::Borrowed(&[ProtocolVersion::V_2026_07_28])
        } else {
            Cow::Borrowed(&[ProtocolVersion::V_2025_11_25, ProtocolVersion::V_2026_07_28])
        }
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.iter().find(|t| t.name == name).cloned()
    }
    async fn list_prompts(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Err(ErrorData::method_not_found::<ListPromptsRequestMethod>())
    }
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        no_cursor(request)?;
        Ok(ListToolsResult {
            tools: self.tools.as_ref().clone(),
            ttl_ms: cached(&context).then_some(0),
            cache_scope: cached(&context).then_some(CacheScope::Private),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = serde_json::Value::Object(request.arguments.unwrap_or_default());
        let value = match request.name.as_ref() {
            "mo_capabilities" => {
                if serde_json::from_value::<Empty>(args).is_err() {
                    return Ok(failed(Failure::new(
                        FailureCode::InputInvalid,
                        "capabilities accepts no arguments",
                    )));
                }
                let limits = mo_embedded_sdk::delivery::DeliveryLimits::default();
                let mut capabilities = serde_json::json!({"contractVersion":"musteroffice.computation/1-draft","adapter":"musteroffice.local-files/1-draft","operations":["create","import","describeTemplate","instantiateTemplate","apply","export"],"exportRenderer":self.bridge.renderer(),"schemas":SchemaId::ALL,"businessJobs":false,"productCommitted":false,"completePresentationCapability":false,"targetApplicationValidated":false,"fileChannel":{"names":"portable ASCII leaf names, at most 128 bytes","inputs":"caller-configured input directory; manifest entries are relative leaf names in it","outputs":"caller-configured output directory; each call exclusively creates its requested child","temporary":"caller-configured separate protected directory","resourceChunkBytes":resources::CHUNK_BYTES,"invocationBytes":mo_embedded_sdk::operation::MAX_INVOCATION_BYTES,"maxInputAssets":limits.max_artifacts,"maxInputAssetBytes":limits.max_asset_bytes,"maxInputTotalBytes":limits.max_total_bytes,"maxFontBytes":limits.max_font_bytes,"spoolWaitMillis":mo_native_compute::SPOOL_WAIT.as_millis(),"cleanup":"release attempts retain the slot for the bounded spool wait even after calculation cancellation"},"cancellation":"cooperative computation and IO; no persistent task survives restart"});
                if self.http {
                    capabilities["transport"] = serde_json::json!({"kind":"streamableHttp","protocols":["2026-07-28"],"sessions":false,"inputBytes":crate::transport::INPUT_BYTES,"responseBytes":crate::transport::OUTPUT_BYTES,"resources":"caller-mounted files; attachments and authorization belong to the gateway","cancellation":"closing this HTTP response cancels this request"});
                }
                capabilities
            }
            "mo_schema" => {
                let arg: SchemaArgument = match serde_json::from_value(args) {
                    Ok(a) => a,
                    Err(_) => {
                        return Ok(failed(Failure::new(
                            FailureCode::InputInvalid,
                            "invalid schema identifier",
                        )));
                    }
                };
                match arg.id.document() {
                    Ok(schema) => serde_json::to_value(schema).expect("schema JSON"),
                    Err(e) => return Ok(failed(e)),
                }
            }
            "mo_presentations_compute" => {
                let args: FileArguments = match serde_json::from_value(args) {
                    Ok(a) => a,
                    Err(_) => {
                        return Ok(failed(Failure::new(
                            FailureCode::InputInvalid,
                            "invalid file computation arguments",
                        )));
                    }
                };
                let result = match self.bridge.compute(args, context).await {
                    Ok(result) => result,
                    Err(error) => return Ok(failed(error)),
                };
                let path = result
                    .result_file
                    .to_str()
                    .expect("portable output name")
                    .replace('\\', "/");
                let resource =
                    Resource::new(format!("{}{path}", resources::PREFIX), "computation-result")
                        .with_mime_type("application/json")
                        .with_size(result.result_byte_length.get());
                let mut response = CallToolResult::structured(
                    serde_json::to_value(Response::Computed { result }).expect("response JSON"),
                );
                response.is_error = Some(false);
                response.content.push(ContentBlock::ResourceLink(resource));
                return Ok(response.into());
            }
            _ => {
                return Err(ErrorData::invalid_params(
                    "Unknown or unavailable tool",
                    None,
                ));
            }
        };
        Ok(CallToolResult::structured(value).into())
    }
    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        no_cursor(request)?;
        Ok(ListResourcesResult {
            ttl_ms: cached(&context).then_some(0),
            cache_scope: cached(&context).then_some(CacheScope::Private),
            ..Default::default()
        })
    }
    async fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        no_cursor(request)?;
        Ok(ListResourceTemplatesResult{resource_templates:vec![ResourceTemplate::new("musteroffice://output/{directory}/{file}{?offset,length}","caller-output").with_description("Caller-owned output bytes. No recursive listing or document store. Full reads <=256 KiB; larger files use offset/length ranges or the native file channel. Outputs must remain unchanged while consumed.")],ttl_ms:cached(&context).then_some(0),cache_scope:cached(&context).then_some(CacheScope::Private),..Default::default()})
    }
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let caching = cached(&context);
        let content = self.bridge.read(request.uri, context).await.map_err(|e| {
            let code = match e.code {
                FailureCode::NotFound if !caching => -32002,
                FailureCode::NotFound | FailureCode::InputInvalid | FailureCode::LimitExceeded => {
                    -32602
                }
                _ => -32603,
            };
            ErrorData::new(
                ErrorCode(code),
                "Output resource unavailable",
                serde_json::to_value(e).ok(),
            )
        })?;
        let mut result = ReadResourceResult::new(vec![content]);
        if caching {
            result = result.with_ttl_ms(0).with_cache_scope(CacheScope::Private);
        }
        Ok(result.into())
    }
}
