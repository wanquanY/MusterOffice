//! Bounded native execution over the same durable StandardHost. No protocol,
//! persistent second queue, implicit identity or connection-scoped job lifetime.
mod config;
mod scheduler;
use crate::StandardHost;
pub use config::{HostClock, RuntimeOptions, StandardHostConfig};
use mo_operation_service::*;
use scheduler::Scheduler;
use std::time::Instant;

pub struct NativeRuntime {
    config: StandardHostConfig,
    context: CallContext,
    options: RuntimeOptions,
    clock: HostClock,
    scheduler: Scheduler,
}
impl NativeRuntime {
    pub fn start(
        config: StandardHostConfig,
        context: CallContext,
        options: RuntimeOptions,
        clock: HostClock,
    ) -> Result<Self, Failure> {
        let options = options.validate()?;
        let scheduler = Scheduler::start(&config, context.clone(), options, clock.clone())?;
        Ok(Self {
            config,
            context,
            options,
            clock,
            scheduler,
        })
    }
    /// Each control session owns a separate connection. Computation is always
    /// performed by the bounded workers, including sync/auto requests.
    pub fn connect(&self) -> Result<NativeSession<'_>, Failure> {
        Ok(NativeSession {
            runtime: self,
            host: self.config.connect()?,
        })
    }
    pub fn health(&self) -> Result<(), Failure> {
        self.scheduler.health()
    }
    /// Stops new scheduling iterations, then joins. Each worker already in an
    /// iteration may claim and finish at most one further job before exiting.
    /// Queued work remains durable. Shutdown does not cancel business jobs.
    pub fn shutdown(mut self) -> Result<(), Failure> {
        self.scheduler.shutdown()
    }
}

pub struct NativeSession<'r> {
    runtime: &'r NativeRuntime,
    host: StandardHost,
}
impl NativeSession<'_> {
    /// Authorized binary port for protocol/SDK adapters. The trusted runtime
    /// supplies identity; resource IDs never grant access by themselves.
    pub fn open_asset(&self, id: &AssetId) -> Result<crate::AssetReader<'_>, Failure> {
        self.host.open_asset(&self.runtime.context, id)
    }
    pub fn append_upload(
        &mut self,
        id: &UploadId,
        offset: mo_common::ByteLength,
        bytes: &[u8],
    ) -> Result<UploadInfo, Failure> {
        self.host.append_upload(
            &self.runtime.context,
            id,
            offset,
            bytes,
            (self.runtime.clock)(),
        )
    }
    pub fn capabilities(&self) -> HostCapabilities {
        let mut capabilities = self.host.capabilities(&self.runtime.context);
        capabilities.queued_execution = JobExecution::HostScheduled;
        if self.runtime.health().is_err() {
            for operation in &mut capabilities.operations {
                if operation.profile_id.is_some() {
                    operation.available = false;
                    operation.unavailable_reason = Some(UnavailableReason::ExecutionUnavailable);
                }
            }
        }
        capabilities
    }
    pub fn dispatch(&mut self, request: HostRequest) -> HostResponse {
        let result = (|| match request {
            HostRequest::Capabilities {} => Ok(HostResponse::Succeeded {
                result: HostResult::Capabilities {
                    capabilities: Box::new(self.capabilities()),
                },
            }),
            HostRequest::Submit { request } => {
                let mode = request.output_mode;
                let mut job = self.host.submit_with_admission(
                    &self.runtime.context,
                    *request,
                    (self.runtime.clock)(),
                    &|| self.runtime.health(),
                )?;
                self.runtime.scheduler.notify();
                if mode != OutputMode::Job {
                    let deadline = Instant::now() + self.runtime.options.sync_wait;
                    while !job.state.terminal() && Instant::now() < deadline {
                        let generation = self.runtime.scheduler.generation();
                        job = self.host.operation_status(
                            &self.runtime.context,
                            &job.id,
                            (self.runtime.clock)(),
                        )?;
                        if job.state.terminal() || self.runtime.health().is_err() {
                            break;
                        }
                        self.runtime.scheduler.wait(
                            generation,
                            deadline
                                .saturating_duration_since(Instant::now())
                                .min(self.runtime.options.idle_poll),
                        );
                    }
                }
                Ok(HostResponse::job(job))
            }
            request => Ok(self.host.dispatch(
                &self.runtime.context,
                request,
                self.runtime.clock.as_ref(),
                &|| false,
            )),
        })();
        result.unwrap_or_else(|error| HostResponse::Failed { error, job: None })
    }
    pub fn dispatch_json(&mut self, input: &str) -> String {
        let result = match decode_host_request(input) {
            Ok(request) => self.dispatch(request),
            Err(error) => HostResponse::Failed { error, job: None },
        };
        serde_json::to_string(&result).expect("typed host response")
    }
}
