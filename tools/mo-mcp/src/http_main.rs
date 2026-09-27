//! Local development listener. Public deployment belongs to the caller's
//! authenticated gateway; the library exposes the same endpoint for embedding.
use axum::{Router, body::Body, extract::Request, routing::any};
use mo_mcp::{
    compute::Config,
    http::{Endpoint, Options},
};
use std::{net::SocketAddr, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: mo-mcp-http <caller-file-config.json> <loopback-ip:port>".into());
    }
    let config = Config::read(&PathBuf::from(&args[0]))?;
    let addr: SocketAddr = args[1].to_str().ok_or("address must be UTF-8")?.parse()?;
    if !addr.ip().is_loopback() {
        return Err("development HTTP listener requires loopback; use the embedded endpoint behind the product gateway".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(config.computation_slots() + config.control_slots() + 2)
        .build()?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let local = listener.local_addr()?;
        let endpoint = Endpoint::new(
            &config,
            Options {
                allowed_hosts: vec![local.to_string()],
                allowed_origins: vec![],
            },
        )?;
        let handler = endpoint.clone();
        let router = Router::new().route(
            "/mcp",
            any(move |request: Request<Body>| {
                let endpoint = handler.clone();
                async move { endpoint.handle(request).await }
            }),
        );
        println!(
            "{}",
            serde_json::json!({
                "url": format!("http://{local}/mcp"),
                "protocol": "2026-07-28",
                "scope": "caller-mounted files; loopback development listener"
            })
        );
        let shutdown = endpoint.clone();
        let result = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                shutdown.shutdown();
            })
            .await;
        endpoint.shutdown();
        result.map_err(Box::<dyn std::error::Error>::from)
    })
}
