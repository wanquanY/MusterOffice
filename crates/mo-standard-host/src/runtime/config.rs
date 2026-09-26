use crate::{HostLimits, StandardHost};
use mo_common::Digest;
use mo_operation_service::{Failure, FailureCode, UnixMillis};
use mo_presentation_delivery::PreviewRenderer;
use std::{path::PathBuf, sync::Arc, time::Duration};

type RendererFactory = dyn Fn() -> Result<Box<dyn PreviewRenderer + Send>, Failure> + Send + Sync;
pub type HostClock = Arc<dyn Fn() -> UnixMillis + Send + Sync>;

/// Trusted process configuration, never deserialized from operation arguments.
#[derive(Clone)]
pub struct StandardHostConfig {
    database: PathBuf,
    executor: Digest,
    limits: HostLimits,
    renderer: Option<Arc<RendererFactory>>,
}
impl StandardHostConfig {
    pub fn new(database: PathBuf, executor: Digest, limits: HostLimits) -> Self {
        Self {
            database,
            executor,
            limits,
            renderer: None,
        }
    }
    pub fn with_preview_renderer(
        mut self,
        factory: impl Fn() -> Result<Box<dyn PreviewRenderer + Send>, Failure> + Send + Sync + 'static,
    ) -> Self {
        self.renderer = Some(Arc::new(factory));
        self
    }
    pub fn connect(&self) -> Result<StandardHost, Failure> {
        let mut host = StandardHost::open(&self.database, self.executor.clone(), self.limits)?;
        if let Some(factory) = &self.renderer {
            host.set_preview_renderer(factory()?);
        }
        Ok(host)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeOptions {
    pub workers: usize,
    pub idle_poll: Duration,
    pub recovery_batch: u32,
    /// Maximum condition-variable wait budget; database calls retain their
    /// separately configured busy timeout. Expiry returns the durable job.
    pub sync_wait: Duration,
}
impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            workers: 2,
            idle_poll: Duration::from_millis(250),
            recovery_batch: 32,
            sync_wait: Duration::from_secs(1),
        }
    }
}
impl RuntimeOptions {
    pub(super) fn validate(self) -> Result<Self, Failure> {
        if !(1..=16).contains(&self.workers)
            || !(Duration::from_millis(10)..=Duration::from_secs(60)).contains(&self.idle_poll)
            || !(1..=256).contains(&self.recovery_batch)
            || self.sync_wait > Duration::from_secs(30)
        {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid native runtime limits",
            ));
        }
        Ok(self)
    }
}
