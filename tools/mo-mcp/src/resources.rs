//! Resource names locate content within an originating server; they never grant
//! authority. Every asset read still goes through the injected host context.
use crate::{bridge::HostBridge, transport};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use mo_operation_service::*;
use rmcp::{ErrorData, model::*};

pub const INLINE_ASSET_BYTES: u64 = 1024 * 1024;
pub const LIMITS_URI: &str = "musteroffice://adapter/limits";
const SCHEMA_PREFIX: &str = "musteroffice://kernel/schemas/";

pub struct ResourceSpace {
    asset_prefix: String,
    read_assets: bool,
}
impl ResourceSpace {
    pub fn new(context: &CallContext) -> Self {
        let namespace = mo_common::digest(
            "musteroffice.mcp-resource-scope/1",
            &(&context.scope, &context.principal),
        )
        .expect("identity serialization");
        Self {
            asset_prefix: format!("musteroffice://scope-{namespace}/assets/"),
            read_assets: context.permissions.contains(&Permission::ReadAssets),
        }
    }
    pub fn list(&self) -> Vec<Resource> {
        let mut items = vec![
            Resource::new(LIMITS_URI, "adapter-limits")
                .with_description("Actual MCP transport limits, separate from kernel/host limits.")
                .with_mime_type("application/json"),
        ];
        for id in SchemaId::ALL {
            let name = serde_json::to_value(id).unwrap();
            let name = name.as_str().unwrap();
            items.push(
                Resource::new(format!("{SCHEMA_PREFIX}{name}"), name)
                    .with_mime_type("application/json"),
            );
        }
        items.sort_by(|a, b| a.uri.cmp(&b.uri));
        items
    }
    pub fn templates(&self) -> Vec<ResourceTemplate> {
        if !self.read_assets {
            return vec![];
        }
        vec![ResourceTemplate::new(format!("{}{{assetId}}", self.asset_prefix), "scoped-asset")
            .with_description("Immutable authorized bytes up to 1 MiB. Larger assets return a descriptor for the operator-authorized native data channel.")]
    }
    pub fn asset(&self, id: &AssetId, media_type: &str, length: u64) -> Resource {
        Resource::new(format!("{}{id}", self.asset_prefix), id.as_str())
            .with_mime_type(if length <= INLINE_ASSET_BYTES {
                media_type
            } else {
                "application/json"
            })
            .with_description(if length <= INLINE_ASSET_BYTES {
                "Authorized immutable asset bytes."
            } else {
                "Large asset transfer descriptor; original bytes use the native data channel."
            })
    }
    pub fn links(&self, response: &HostResponse) -> Vec<ContentBlock> {
        if !self.read_assets {
            return vec![];
        }
        let HostResponse::Succeeded { result } = response else {
            return vec![];
        };
        let assets = match result {
            HostResult::Asset { asset } => vec![self.asset(
                &asset.id,
                &asset.descriptor.media_type,
                asset.descriptor.byte_length.get(),
            )],
            HostResult::Upload { upload } => upload
                .asset
                .iter()
                .map(|a| {
                    self.asset(
                        &a.id,
                        &a.descriptor.media_type,
                        a.descriptor.byte_length.get(),
                    )
                })
                .collect(),
            HostResult::Job { job } => match &job.result {
                Some(TerminalResult::Succeeded { receipt }) => match receipt.as_ref() {
                    OperationReceipt::Export(export) => export
                        .bundle
                        .assets
                        .iter()
                        .map(|a| {
                            self.asset(
                                &AssetId::new(a.id.as_str()).expect("shared ID grammar"),
                                &a.media_type,
                                a.byte_length.get(),
                            )
                        })
                        .collect(),
                    _ => vec![],
                },
                _ => vec![],
            },
            _ => vec![],
        };
        assets
            .into_iter()
            .map(ContentBlock::resource_link)
            .collect()
    }
    pub async fn read(&self, uri: &str, host: &HostBridge) -> Result<ResourceContents, ErrorData> {
        if uri == LIMITS_URI {
            return Ok(json_content(
                uri,
                serde_json::json!({
                    "version":"musteroffice.mcp-stdio/1-draft",
                    "inputBytes":transport::INPUT_BYTES,"outputBytes":transport::OUTPUT_BYTES,
                    "inFlightRequests":transport::REQUESTS,"inFlightNotifications":transport::NOTIFICATIONS,
                    "cancelledRequestHistory":transport::CANCEL_HISTORY,
                    "inlineAssetBytes":INLINE_ASSET_BYTES,"writeDeadlineMs":10000,
                    "protocolErrorSlots":1,
                    "invalidMessage":"bounded parse/invalid-request/invalid-params response; valid notifications get no response",
                    "fatalInput":"oversize, unterminated, unsolicited response, ambiguous active identity or admission exhaustion",
                    "overload":"connection-closed; durable accepted jobs remain in the host",
                    "binaryChannel":"operator-authorized native host append/read-asset; no arbitrary paths in tool arguments",
                    "rssBoundProven":false,"completePresentationAcceptance":false
                }),
            ));
        }
        if let Some(name) = uri.strip_prefix(SCHEMA_PREFIX) {
            let id: SchemaId = serde_json::from_value(name.into()).map_err(|_| unknown())?;
            let response = host
                .dispatch(HostRequest::GetSchema { id })
                .await
                .map_err(failure)?;
            return match response {
                HostResponse::Succeeded {
                    result: HostResult::Schema { document },
                } => Ok(json_content(
                    uri,
                    serde_json::to_value(document).expect("schema JSON"),
                )),
                HostResponse::Failed { error, .. } => Err(failure(error)),
                _ => Err(ErrorData::internal_error("unexpected schema result", None)),
            };
        }
        let id = uri
            .strip_prefix(&self.asset_prefix)
            .filter(|s| s.len() <= 128)
            .and_then(|s| AssetId::new(s).ok())
            .ok_or_else(unknown)?;
        if !self.read_assets {
            return Err(unknown());
        }
        let response = host
            .dispatch(HostRequest::ReadAsset {
                asset_id: id.clone(),
            })
            .await
            .map_err(failure)?;
        let info = match response {
            HostResponse::Succeeded {
                result: HostResult::Asset { asset },
            } => asset,
            HostResponse::Failed { error, .. } => return Err(failure(error)),
            _ => return Err(ErrorData::internal_error("unexpected asset result", None)),
        };
        if info.descriptor.byte_length.get() > INLINE_ASSET_BYTES {
            return Ok(json_content(
                uri,
                serde_json::json!({"kind":"assetTransfer", "asset":info,
                "channel":"authorized-native-host", "operation":"read-asset",
                "inlineByteLimit":INLINE_ASSET_BYTES,
                "requires":"operator-bound database, principal and scope; verify length and SHA256 after download"}),
            ));
        }
        let asset = host
            .read_asset(id, INLINE_ASSET_BYTES)
            .await
            .map_err(failure)?;
        Ok(ResourceContents::blob(STANDARD.encode(asset.bytes), uri)
            .with_mime_type(asset.info.descriptor.media_type))
    }
}
fn json_content(uri: &str, value: serde_json::Value) -> ResourceContents {
    ResourceContents::text(value.to_string(), uri).with_mime_type("application/json")
}
fn unknown() -> ErrorData {
    ErrorData::new(
        ErrorCode(-32002),
        "Resource not found in this server scope",
        None,
    )
}
fn failure(error: Failure) -> ErrorData {
    let code = if matches!(
        error.code,
        FailureCode::NotFound | FailureCode::NotAuthorized
    ) {
        -32002
    } else {
        -32603
    };
    ErrorData::new(
        ErrorCode(code),
        "Resource read failed",
        Some(serde_json::to_value(error).expect("failure JSON")),
    )
}
