//! Optional HTTP transport over caller-owned file computation. The embedding
//! gateway authenticates every request and selects this isolated endpoint.
mod body;
#[cfg(test)]
mod tests;
use crate::{
    compute::{Bridge, ComputeServer, Config},
    transport::{INPUT_BYTES, OUTPUT_BYTES},
};
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub const HTTP_REQUESTS: usize = 8;
pub const BODY_TIMEOUT: Duration = Duration::from_secs(10);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

/// Host and Origin values come from trusted deployment configuration. An empty
/// origin list rejects every browser Origin; it does not disable validation.
pub struct Options {
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
}
impl Options {
    fn validate(&self) -> Result<(), &'static str> {
        if self.allowed_hosts.is_empty()
            || self.allowed_hosts.len() > 32
            || self.allowed_origins.len() > 32
        {
            return Err("HTTP host/origin list limit");
        }
        for host in &self.allowed_hosts {
            if host.len() > 256
                || host.contains(['*', '@'])
                || host.parse::<axum::http::uri::Authority>().is_err()
            {
                return Err("HTTP host must be an explicit authority");
            }
        }
        for origin in &self.allowed_origins {
            let uri = origin
                .parse::<axum::http::Uri>()
                .map_err(|_| "invalid HTTP origin")?;
            if origin.len() > 512
                || !matches!(uri.scheme_str(), Some("http" | "https"))
                || uri.authority().is_none()
                || origin.contains(['*', '@', '?', '#'])
                || uri.path() != "/"
            {
                return Err("HTTP origin must be an explicit http(s) origin");
            }
        }
        Ok(())
    }
}
struct State {
    bridge: Arc<Bridge>,
    server: ComputeServer,
    config: StreamableHttpServerConfig,
    requests: Arc<Semaphore>,
    stopped: CancellationToken,
}
impl Drop for State {
    fn drop(&mut self) {
        self.bridge.stop();
        self.stopped.cancel();
    }
}
/// One caller-authorized file namespace, not a tenant registry or OAuth server.
/// Products mount an instance behind their gateway and retain it between calls.
#[derive(Clone)]
pub struct Endpoint {
    state: Arc<State>,
}
impl Endpoint {
    pub fn new(files: &Config, options: Options) -> Result<Self, Box<dyn std::error::Error>> {
        options.validate()?;
        let bridge = Arc::new(Bridge::new(files)?);
        let stopped = CancellationToken::new();
        let config = StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_stateless_protocol_metadata_required(true)
            .with_json_response(false)
            .with_max_request_body_bytes(INPUT_BYTES)
            .with_allowed_hosts(options.allowed_hosts)
            .with_allowed_origins(options.allowed_origins)
            .enforce_origin_validation()
            .with_sse_keep_alive(Some(Duration::from_secs(1)))
            .with_cancellation_token(stopped.clone());
        Ok(Self {
            state: Arc::new(State {
                server: ComputeServer::http(bridge.clone()),
                bridge,
                config,
                requests: Arc::new(Semaphore::new(HTTP_REQUESTS)),
                stopped,
            }),
        })
    }
    /// Stops new requests and signals actual computations. The embedding runtime
    /// must remain alive to drain its blocking work and bounded cleanup.
    pub fn shutdown(&self) {
        self.state.bridge.stop();
        self.state.stopped.cancel();
    }
    pub async fn handle(&self, request: Request<Body>) -> Response<Body> {
        if self.state.stopped.is_cancelled() {
            return error(StatusCode::SERVICE_UNAVAILABLE, "endpoint stopped");
        }
        let permit = match self.state.requests.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => return error(StatusCode::TOO_MANY_REQUESTS, "HTTP in-flight limit"),
        };
        if request.headers().len() > 64
            || request
                .headers()
                .iter()
                .map(|(n, v)| n.as_str().len() + v.as_bytes().len())
                .sum::<usize>()
                > 16384
        {
            return error(
                StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                "HTTP header budget",
            );
        }
        for name in [
            "origin",
            "host",
            "content-type",
            "mcp-protocol-version",
            "mcp-method",
            "mcp-name",
        ] {
            if request.headers().get_all(name).iter().count() > 1 {
                return error(
                    if name == "origin" {
                        StatusCode::FORBIDDEN
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "ambiguous HTTP header",
                );
            }
        }
        let (parts, input) = request.into_parts();
        // The SDK validates Host/Origin/method before consuming this bounded,
        // duplicate-key-rejecting body; no document data is opened at this stage.
        let (input, input_failure) = body::validated_input(input);
        let request = Request::from_parts(parts, input);
        let server = self.state.server.clone();
        // The upstream SDK lazily caches schemas even for unknown tool names.
        // Scope that routing cache to one request, while sharing immutable tools,
        // computation permits and files. There is no protocol session to retain.
        let service = StreamableHttpService::new(
            move || Ok(server.clone()),
            Arc::new(NeverSessionManager::default()),
            self.state.config.clone(),
        );
        let deadline = tokio::time::Instant::now() + REQUEST_TIMEOUT;
        let response = tokio::select! {
            _=self.state.stopped.cancelled()=>return error(StatusCode::SERVICE_UNAVAILABLE,"endpoint stopped"),
            result=tokio::time::timeout_at(deadline,service.handle(request))=>match result {
                Ok(r)=>r.map(Body::new),
                Err(_)=>return error(StatusCode::GATEWAY_TIMEOUT,"HTTP request deadline"),
            },
        };
        // Input policy belongs to this adapter. The SDK sees Body::Error as an
        // internal I/O error; retain our typed cause and return the actual client
        // timeout/limit/validation status instead of misreporting a server fault.
        if let Some(failure) = input_failure.get() {
            return failure.response();
        }
        let (mut parts, output) = response.into_parts();
        parts
            .headers
            .insert("cache-control", "no-store".parse().expect("static header"));
        parts.headers.insert(
            "x-content-type-options",
            "nosniff".parse().expect("static header"),
        );
        if parts
            .headers
            .get("content-type")
            .is_some_and(|v| v.as_bytes().starts_with(b"text/event-stream"))
        {
            parts
                .headers
                .insert("x-accel-buffering", "no".parse().expect("static header"));
        }
        Response::from_parts(
            parts,
            Body::new(body::Output::new(
                output,
                permit,
                self.state.clone(),
                OUTPUT_BYTES,
                deadline,
            )),
        )
    }
}
fn error(status: StatusCode, message: &'static str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .header("cache-control", "no-store")
        .body(Body::from(message))
        .expect("static response")
}
