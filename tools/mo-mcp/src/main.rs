use mo_mcp::{
    compute::{Bridge, ComputeServer, Config},
    transport::BoundedTransport,
};
use rmcp::ServiceExt;
use std::{path::PathBuf, sync::Arc};
mod stdio;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let config = match args.as_slice() {
        [path] => Config::read(&PathBuf::from(path))?,
        [mode, package, caller] if mode == "--package" =>
            Config::from_package(&PathBuf::from(package), &PathBuf::from(caller))?,
        _ => return Err("usage: mo-mcp <local-file-config.json> | mo-mcp --package <runtime.json> <caller-file-config.json>".into()),
    };
    let bridge = Arc::new(Bridge::new(&config)?);
    let server = ComputeServer::new(bridge.clone());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(
            config.control_slots() + config.computation_slots() + stdio::BLOCKING_THREADS,
        )
        .build()?;
    let result: Result<(), Box<dyn std::error::Error>> = runtime.block_on(async {
        let (input, output) = stdio::open()?;
        let (transport, observer) = BoundedTransport::new(input, output);
        let result = match server.serve(transport).await {
            Ok(service) => {
                let waiting = service.waiting();
                tokio::pin!(waiting);
                let done = tokio::select! {
                    done=&mut waiting=>done,
                    _=observer.disconnected()=>{bridge.stop();waiting.await},
                };
                done.map(|_| ()).map_err(|e| e.to_string())
            }
            Err(error) => Err(error.to_string()),
        };
        if let Some(error) = observer.error() {
            return Err(error.into());
        }
        result.map_err(Into::into)
    });
    // EOF, transport failure or shutdown cancels real work before draining the
    // blocking pool. There are no accepted jobs or internal database to resume.
    bridge.stop();
    drop(runtime);
    result
}
