use super::{Config, FileArguments, config::Files, resources};
use mo_embedded_sdk::operation::{Failure, FailureCode};
use mo_native_compute::{FileCall, SavedComputation, workspace::leaf};
use rmcp::{RoleServer, model::ResourceContents, service::RequestContext};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub struct Bridge {
    files: Arc<Files>,
    compute: Arc<Semaphore>,
    control: Arc<Semaphore>,
    stopped: CancellationToken,
}

impl Bridge {
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            files: Arc::new(config.open()?),
            compute: Arc::new(Semaphore::new(config.computation_slots())),
            control: Arc::new(Semaphore::new(config.control_slots())),
            stopped: CancellationToken::new(),
        })
    }
    pub fn stop(&self) {
        self.stopped.cancel();
    }
    pub fn renderer(&self) -> Option<mo_embedded_sdk::delivery::RendererIdentity> {
        self.files.exporter.as_ref().map(|e| e.renderer_identity())
    }
    async fn run<T: Send + 'static>(
        &self,
        context: RequestContext<RoleServer>,
        computing: bool,
        f: impl FnOnce(&Files, &dyn Fn() -> bool) -> Result<T, Failure> + Send + 'static,
    ) -> Result<T, Failure> {
        let slots = if computing {
            &self.compute
        } else {
            &self.control
        };
        let permit = slots.clone().try_acquire_owned().map_err(|_| {
            Failure::new(
                FailureCode::LimitExceeded,
                "MCP active computation/read limit",
            )
        })?;
        let files = self.files.clone();
        let stop = self.stopped.clone();
        let token = context.ct.child_token();
        // The SDK may drop a cancelled handler. Its blocking computation must
        // receive cancellation and keep its slot/context until cleanup ends.
        let _guard = token.clone().drop_guard();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _context = context;
            let cancelled = || token.is_cancelled() || stop.is_cancelled();
            check(&cancelled)?;
            f(&files, &cancelled)
        })
        .await
        .map_err(|_| {
            Failure::new(
                FailureCode::ExecutionInterrupted,
                "MCP computation interrupted",
            )
        })?
    }
    pub async fn compute(
        &self,
        args: FileArguments,
        context: RequestContext<RoleServer>,
    ) -> Result<SavedComputation, Failure> {
        self.run(context, true, move |files, cancelled| {
            leaf(&args.output_directory).map_err(io_failure)?;
            let output = files
                .outputs
                .child(&args.output_directory)
                .map_err(io_failure)?;
            let mut result = mo_native_compute::compute(
                FileCall {
                    invocation: files
                        .inputs
                        .read(&args.invocation_file)
                        .map_err(io_failure)?,
                    inputs: files.inputs.read(&args.inputs_file).map_err(io_failure)?,
                    temporary_directory: files.temporary.path(),
                    output_directory: &output,
                    exporter: files.exporter.as_ref(),
                },
                &|path| {
                    let name = path
                        .to_str()
                        .ok_or_else(|| std::io::Error::other("UTF-8 file name required"))?;
                    files.inputs.read(name)
                },
                cancelled,
            )?;
            // Never serialize private installation paths into protocol data.
            result.result_file = PathBuf::from(args.output_directory).join("result.json");
            Ok(result)
        })
        .await
    }
    pub async fn read(
        &self,
        uri: String,
        context: RequestContext<RoleServer>,
    ) -> Result<ResourceContents, Failure> {
        self.run(context, false, move |files, cancelled| {
            resources::read(&files.outputs, &uri, cancelled)
        })
        .await
    }
}
pub(super) fn check(cancelled: &dyn Fn() -> bool) -> Result<(), Failure> {
    if cancelled() {
        Err(Failure::new(
            FailureCode::Cancelled,
            "MCP computation cancelled",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn io_failure(e: std::io::Error) -> Failure {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => FailureCode::NotFound,
        std::io::ErrorKind::InvalidInput => FailureCode::InputInvalid,
        _ => FailureCode::IoFailure,
    };
    Failure::new(code, e.to_string())
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop();
    }
}
