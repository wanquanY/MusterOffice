use mo_mcp::{
    bridge::HostBridge, config::OperatorConfig, resources::ResourceSpace, server::OfficeServer,
    transport::BoundedTransport,
};
use rmcp::ServiceExt;
use std::{path::PathBuf, sync::Arc};
mod stdio;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().ok_or("usage: mo-mcp <operator-config.json> [append <upload-id> <offset> | read-asset <asset-id> <offset> <length>]")?);
    let config = OperatorConfig::read(&path)?;
    let remaining: Vec<_> = args.collect();
    if !remaining.is_empty() {
        return mo_mcp::channel::run(&config, &remaining);
    }
    let resources = ResourceSpace::new(&config.context());
    let bridge = Arc::new(HostBridge::new(
        config.start()?,
        config.control_slots,
        config.computation_slots,
    )?);
    let server = OfficeServer::new(bridge.clone(), resources)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(
            config.control_slots + config.computation_slots + stdio::BLOCKING_THREADS,
        )
        .build()?;
    let result: Result<(), Box<dyn std::error::Error>> = runtime.block_on(async {
        let (input, output) = stdio::open()?;
        let (transport, observer) = BoundedTransport::new(input, output);
        let result = match server.serve(transport).await {
            Ok(service) => service
                .waiting()
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Err(error) => Err(error.to_string()),
        };
        if let Some(error) = observer.error() {
            return Err(error.into());
        }
        result.map_err(Into::into)
    });
    // Drain accepted blocking control calls outside the event loop before
    // joining native scheduler workers. Queued jobs remain in the database.
    drop(runtime);
    let shutdown = Arc::try_unwrap(bridge)
        .map_err(|_| "MCP service retained a host reference")?
        .shutdown();
    result?;
    shutdown?;
    Ok(())
}
