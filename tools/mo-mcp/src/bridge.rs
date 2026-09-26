//! Bounded async bridge. A cancelled await never abandons an accepted host job.
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_standard_host::NativeRuntime;
use sha2::{Digest as _, Sha256};
use std::sync::Arc;
use tokio::sync::Semaphore;
#[cfg(test)]
mod tests;

pub struct HostBridge {
    runtime: Arc<NativeRuntime>,
    control: Arc<Semaphore>,
    computation: Arc<Semaphore>,
}
pub struct AssetBytes {
    pub info: AssetInfo,
    pub bytes: Vec<u8>,
}
impl HostBridge {
    pub fn new(
        runtime: NativeRuntime,
        control: usize,
        computation: usize,
    ) -> Result<Self, Failure> {
        if !(1..=8).contains(&control) || !(1..=8).contains(&computation) {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid control bridge concurrency",
            ));
        }
        Ok(Self {
            runtime: Arc::new(runtime),
            control: Arc::new(Semaphore::new(control)),
            computation: Arc::new(Semaphore::new(computation)),
        })
    }
    pub async fn dispatch(&self, request: HostRequest) -> Result<HostResponse, Failure> {
        // A full set of expensive requests cannot occupy the reserved query /
        // cancellation slots. No unbounded queue is placed before spawn_blocking.
        let slots = match request {
            HostRequest::Submit { .. }
            | HostRequest::ReadDocument { .. }
            | HostRequest::SealUpload { .. } => &self.computation,
            _ => &self.control,
        };
        let permit = slots.clone().try_acquire_owned().map_err(|_| busy())?;
        let runtime = self.runtime.clone();
        tokio::task::spawn_blocking(move || {
            // Retain the permit inside the blocking job, including when its
            // async caller disappears. Dropping JoinHandle does not cancel it.
            let _permit = permit;
            let mut session = runtime.connect()?;
            Ok(session.dispatch(request))
        })
        .await
        .map_err(|_| interrupted())?
    }
    pub async fn read_asset(&self, id: AssetId, limit: u64) -> Result<AssetBytes, Failure> {
        let permit = self
            .computation
            .clone()
            .try_acquire_owned()
            .map_err(|_| busy())?;
        let runtime = self.runtime.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let session = runtime.connect()?;
            let reader = session.open_asset(&id)?;
            let n = reader.info().descriptor.byte_length.get();
            if n > limit {
                return Err(Failure::new(
                    FailureCode::LimitExceeded,
                    "asset exceeds MCP inline limit; use the authorized native data channel",
                ));
            }
            let mut bytes = vec![0; usize::try_from(n).map_err(|_| busy())?];
            reader.read_exact_at(&mut bytes, 0).map_err(|_| {
                Failure::new(FailureCode::StorageFailure, "asset bytes unavailable")
            })?;
            if mo_common::Digest::from_sha256(Sha256::digest(&bytes).into())
                != reader.info().descriptor.sha256
            {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "asset byte digest differs from its descriptor",
                ));
            }
            Ok(AssetBytes {
                info: reader.info().clone(),
                bytes,
            })
        })
        .await
        .map_err(|_| interrupted())?
    }
    pub fn capabilities(&self) -> Result<HostCapabilities, Failure> {
        Ok(self.runtime.connect()?.capabilities())
    }
    /// Call after stopping the protocol runtime and draining its blocking pool.
    /// No business cancellation is implied by service shutdown.
    pub fn shutdown(self) -> Result<(), Failure> {
        Arc::try_unwrap(self.runtime)
            .map_err(|_| {
                Failure::new(
                    FailureCode::ExecutionInterrupted,
                    "host bridge still has active calls at shutdown",
                )
            })?
            .shutdown()
    }
}
fn busy() -> Failure {
    Failure::new(
        FailureCode::LimitExceeded,
        "host control concurrency limit; retry with the same requestId",
    )
}
fn interrupted() -> Failure {
    Failure::new(
        FailureCode::ExecutionInterrupted,
        "host control worker interrupted",
    )
}
