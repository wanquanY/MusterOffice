//! Trusted installation configuration, read before accepting protocol traffic.
//! This type is deliberately absent from every tool input schema.
use mo_common::Digest;
use mo_operation_service::*;
use mo_standard_host::{HostLimits, NativeRuntime, RuntimeOptions, StandardHostConfig};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperatorConfig {
    database: PathBuf,
    principal: PrincipalId,
    scope: ScopeId,
    permissions: BTreeSet<Permission>,
    #[serde(default = "two")]
    workers: usize,
    #[serde(default = "two")]
    pub control_slots: usize,
    #[serde(default = "two")]
    pub computation_slots: usize,
    preview_worker: Option<PreviewWorker>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewWorker {
    path: PathBuf,
    sha256: Digest,
}
fn two() -> usize {
    2
}
impl OperatorConfig {
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut text = String::new();
        File::open(path)?.take(65537).read_to_string(&mut text)?;
        if text.len() > 65536 {
            return Err("operator configuration byte limit".into());
        }
        let config: Self = mo_common::from_json_str(&text)?;
        if !config.database.is_absolute()
            || config
                .preview_worker
                .as_ref()
                .is_some_and(|w| !w.path.is_absolute())
            || !(1..=16).contains(&config.workers)
            || !(1..=8).contains(&config.control_slots)
            || !(1..=8).contains(&config.computation_slots)
        {
            return Err("invalid operator paths or concurrency".into());
        }
        Ok(config)
    }
    pub fn context(&self) -> CallContext {
        CallContext {
            principal: self.principal.clone(),
            scope: self.scope.clone(),
            permissions: self.permissions.clone(),
        }
    }
    pub fn host(&self) -> Result<StandardHostConfig, Box<dyn std::error::Error>> {
        let mut hash = Sha256::new();
        let mut binary = File::open(std::env::current_exe()?)?;
        let mut bytes = [0; 65536];
        loop {
            let n = binary.read(&mut bytes)?;
            if n == 0 {
                break;
            }
            hash.update(&bytes[..n]);
        }
        let mut config = StandardHostConfig::new(
            self.database.clone(),
            Digest::from_sha256(hash.finalize().into()),
            HostLimits::default(),
        );
        if let Some(worker) = &self.preview_worker {
            let path = worker.path.clone();
            let sha256 = worker.sha256.clone();
            config = config.with_preview_renderer(move || {
                Ok(Box::new(
                    mo_native_render::NativePreviewRenderer::new(
                        path.clone(),
                        sha256.clone(),
                        Duration::from_secs(60),
                    )
                    .map_err(delivery_failure)?,
                ))
            });
        }
        Ok(config)
    }
    pub fn start(&self) -> Result<NativeRuntime, Box<dyn std::error::Error>> {
        Ok(NativeRuntime::start(
            self.host()?,
            self.context(),
            RuntimeOptions {
                workers: self.workers,
                ..RuntimeOptions::default()
            },
            Arc::new(now),
        )?)
    }
}
pub fn now() -> UnixMillis {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis();
    UnixMillis::new(i64::try_from(millis).expect("clock overflow")).expect("nonnegative clock")
}
