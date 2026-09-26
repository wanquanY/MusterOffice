//! Generic MCP envelope validation followed by the SDK's own parameter types.
//! A protocol fault never enters the business host or the SDK task registry.
use rmcp::model::*;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) enum Decoded {
    Message(Box<ClientJsonRpcMessage>),
    Fault {
        id: Option<RequestId>,
        error: ErrorData,
    },
    Ignore,
    Fatal(&'static str),
}
fn fault(id: Option<RequestId>, code: i32, message: &'static str) -> Decoded {
    Decoded::Fault {
        id,
        error: ErrorData::new(ErrorCode(code), message, None),
    }
}
fn params<T: DeserializeOwned + Into<ClientRequest>>(
    raw: Value,
) -> Result<ClientRequest, serde_json::Error> {
    serde_json::from_value::<T>(raw).map(Into::into)
}
fn request(raw: Value, method: &str) -> Result<ClientRequest, serde_json::Error> {
    match method {
        "initialize" => params::<InitializeRequest>(raw),
        "ping" => params::<PingRequest>(raw),
        "server/discover" => params::<DiscoverRequest>(raw),
        "tools/list" => params::<ListToolsRequest>(raw),
        "tools/call" => params::<CallToolRequest>(raw),
        "resources/list" => params::<ListResourcesRequest>(raw),
        "resources/templates/list" => params::<ListResourceTemplatesRequest>(raw),
        "resources/read" => params::<ReadResourceRequest>(raw),
        // Unknown/unadvertised methods use the SDK's method-not-found handler.
        // Do not inherit accidental defaults for logging, prompts or tasks.
        _ => params::<CustomRequest>(raw),
    }
}
pub(super) fn decode(bytes: &[u8]) -> Decoded {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return fault(None, -32700, "Parse error"),
    };
    let raw: Value = match mo_common::from_json_str(text) {
        Ok(raw) => raw,
        Err(error) => {
            return match error.classify() {
                serde_json::error::Category::Data => fault(None, -32600, "Ambiguous JSON object"),
                _ => fault(None, -32700, "Parse error"),
            };
        }
    };
    let Some(object) = raw.as_object() else {
        return fault(None, -32600, "MCP message must be an object");
    };
    // No request is ever sent to the client by this adapter. A response gets
    // no response of its own; ending the connection avoids an error loop.
    if object.contains_key("result") || object.contains_key("error") {
        return Decoded::Fatal("unsolicited client response");
    }
    let id = match object.get("id") {
        None => None,
        Some(value) => {
            if value.to_string().len() > 256 {
                return Decoded::Fatal("request identity byte budget");
            }
            match serde_json::from_value(value.clone()) {
                Ok(id) => Some(id),
                Err(_) => return fault(None, -32600, "Invalid request identity"),
            }
        }
    };
    if object.get("jsonrpc") != Some(&Value::String("2.0".into())) {
        return fault(id, -32600, "Invalid JSON-RPC version");
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return fault(id, -32600, "Invalid request method");
    };
    if object.get("params").is_some_and(|p| !p.is_object()) {
        return if id.is_some() {
            fault(id, -32602, "MCP params must be an object")
        } else {
            Decoded::Ignore
        };
    }
    let method = method.to_owned();
    match id {
        Some(id) => match request(raw, &method) {
            Ok(request) => Decoded::Message(Box::new(JsonRpcMessage::request(request, id))),
            Err(_) => fault(Some(id), -32602, "Invalid method parameters"),
        },
        None => {
            // Valid notification envelopes never get a reply, even if their
            // method parameters are unknown or malformed.
            let notification = match method.as_str() {
                "notifications/initialized" => {
                    serde_json::from_value::<InitializedNotification>(raw).map(Into::into)
                }
                "notifications/cancelled" => {
                    serde_json::from_value::<CancelledNotification>(raw).map(Into::into)
                }
                _ => return Decoded::Ignore,
            };
            match notification {
                Ok(notification) => {
                    Decoded::Message(Box::new(JsonRpcMessage::notification(notification)))
                }
                Err(_) => Decoded::Ignore,
            }
        }
    }
}
