//! Physical execution ownership shared by the host and its worker. Recovery
//! uses OS leases, never process IDs, file ages or application task status.
mod locks;
mod platform;
use locks::FileLock;
use std::{
    fs::{self, TryLockError},
    io,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const NAMESPACE: &str = ".mo-executions-v1";
const REGISTRY: &str = "registry.lock";
const LEASE: &str = "participants.lock";
const MAX_ENTRIES: usize = 1024;
const MAX_RECLAIMS: usize = 64;

/// Private host configuration. The operator must protect this root from other
/// writers and must not rename/remove it while participants are alive. This
/// bounds tracked executions, not aggregate disk bytes or application quotas.
#[derive(Clone)]
pub struct ExecutionSpoolRoot {
    path: PathBuf,
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct SpoolRecovery {
    pub inspected: usize,
    pub live: usize,
    pub reclaimed: usize,
    pub unrecognized: usize,
    pub more_reclaimable: bool,
}

/// Keep this owner after the worker exits for as long as returned readers may
/// access this execution. It is deliberately not Clone or serializable.
pub struct ExecutionSpool {
    root: ExecutionSpoolRoot,
    path: Option<PathBuf>,
    lease: Option<FileLock>,
}

/// A worker must hold this lease before reading inputs or writing any spool.
/// It does not grant document access, execute a task or publish a result.
pub struct ExecutionSpoolLease {
    _lease: FileLock,
}

impl ExecutionSpoolRoot {
    /// Bounded, cooperative acquisition for an active calculation. Low-level
    /// maintenance can continue using the immediate `create`/`recover` APIs.
    pub fn create_with_wait(
        &self,
        timeout: Duration,
        cancelled: &dyn Fn() -> bool,
    ) -> io::Result<ExecutionSpool> {
        wait(timeout, cancelled, || self.create())
    }
    pub fn open(operator_root: &Path) -> io::Result<Self> {
        platform::directory(operator_root)?;
        let parent = fs::canonicalize(operator_root)?;
        let path = parent.join(NAMESPACE);
        match platform::create_directory(&path) {
            Ok(()) => platform::sync_directory(&parent)?,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => platform::directory(&path)?,
            Err(e) => return Err(e),
        }
        let marker = path.join(REGISTRY);
        match platform::regular(&marker, false) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Never recreate a missing registry beside existing executions:
                // another process could still hold the old unlinked lock.
                if fs::read_dir(&path)?.any(|entry| match entry {
                    Ok(entry) => entry.file_name() != REGISTRY,
                    Err(_) => true,
                }) {
                    // A concurrent initializer may already have opened the
                    // registry and started an execution since our first read.
                    platform::regular(&marker, false)?;
                } else {
                    match platform::regular(&marker, true) {
                        Ok(file) => file.sync_all()?,
                        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                            platform::regular(&marker, false)?;
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            Err(e) => return Err(e),
        }
        platform::sync_directory(&path)?;
        Ok(Self { path })
    }

    fn registry(&self) -> io::Result<FileLock> {
        platform::directory(&self.path)?;
        let file = platform::regular(&self.path.join(REGISTRY), false)?;
        Ok(FileLock::try_exclusive(file)?)
    }

    fn entries(&self) -> io::Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if entry.file_name() == REGISTRY {
                continue;
            }
            if entries.len() == MAX_ENTRIES {
                return Err(io::Error::other("execution namespace entry limit exceeded"));
            }
            entries.push(entry.path());
        }
        entries.sort();
        Ok(entries)
    }

    /// Attempts one bounded scan. `WouldBlock` means another host is modifying
    /// the namespace; the host may retry from its own maintenance scheduler.
    pub fn recover(&self, max_reclaims: usize) -> io::Result<SpoolRecovery> {
        if max_reclaims == 0 || max_reclaims > MAX_RECLAIMS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "execution recovery batch limit",
            ));
        }
        let registry = self.registry()?;
        let entries = self.entries()?;
        let report = self.recover_locked(&entries, max_reclaims)?;
        registry.release()?;
        Ok(report)
    }

    fn recover_locked(&self, entries: &[PathBuf], limit: usize) -> io::Result<SpoolRecovery> {
        let mut report = SpoolRecovery::default();
        for path in entries {
            report.inspected += 1;
            let metadata = fs::symlink_metadata(path)?;
            if !execution_name(path) || platform::redirected(&metadata) || !metadata.is_dir() {
                report.unrecognized += 1;
                continue;
            }
            let lock = match platform::regular(&path.join(LEASE), false) {
                Ok(file) => match FileLock::try_exclusive(file) {
                    Ok(lock) => Some(lock),
                    Err(TryLockError::WouldBlock) => {
                        report.live += 1;
                        continue;
                    }
                    Err(TryLockError::Error(e)) => return Err(e),
                },
                // Creation and interrupted cleanup both run under the registry.
                // A missing lease inside this namespace therefore has no live
                // participant that obeys the ownership protocol.
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(e),
            };
            // The registry excludes late joins, including on Windows where
            // removing an open/locked file is not assumed to succeed.
            if let Some(lock) = lock {
                lock.release()?;
            }
            if report.reclaimed == limit {
                report.more_reclaimable = true;
                continue;
            }
            fs::remove_dir_all(path)?;
            report.reclaimed += 1;
            #[cfg(test)]
            tests::pause("reclaimed", path);
        }
        // Also replay the directory barrier after a prior unlink+crash.
        platform::sync_directory(&self.path)?;
        Ok(report)
    }

    /// Attempts allocation and bounded orphan recovery. Contended registry
    /// access returns `WouldBlock`; no file-system wait hides host cancellation.
    pub fn create(&self) -> io::Result<ExecutionSpool> {
        let registry = self.registry()?;
        let entries = self.entries()?;
        let recovered = self.recover_locked(&entries, 16)?;
        if entries.len() - recovered.reclaimed >= MAX_ENTRIES {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "execution spool capacity reached",
            ));
        }
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for _ in 0..128 {
            let nonce = super::NEXT_SPOOL.fetch_add(1, Ordering::Relaxed);
            let path = self
                .path
                .join(format!("execution-{}-{epoch}-{nonce}", std::process::id()));
            match platform::create_directory(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
            #[cfg(test)]
            tests::pause("directory", &path);
            let lease = FileLock::try_shared(platform::regular(&path.join(LEASE), true)?)?;
            lease.sync_all()?;
            platform::sync_directory(&path)?;
            platform::sync_directory(&self.path)?;
            #[cfg(test)]
            tests::pause("leased", &path);
            registry.release()?;
            return Ok(ExecutionSpool {
                root: self.clone(),
                path: Some(path),
                lease: Some(lease),
            });
        }
        Err(io::Error::other("cannot allocate execution identity"))
    }
}

impl ExecutionSpoolLease {
    /// Called by an isolated worker whose parent enforces its deadline. The
    /// shared registry lock can wait only here, inside that killable process.
    pub fn join(path: &Path) -> io::Result<Self> {
        if !execution_name(path) {
            return Err(io::Error::other("execution directory identity"));
        }
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("execution parent"))?;
        if parent.file_name().is_none_or(|name| name != NAMESPACE) {
            return Err(io::Error::other("execution namespace version"));
        }
        platform::directory(parent)?;
        let registry = FileLock::shared(platform::regular(&parent.join(REGISTRY), false)?)?;
        platform::directory(path)?;
        let lease = FileLock::try_shared(platform::regular(&path.join(LEASE), false)?)?;
        registry.release()?;
        Ok(Self { _lease: lease })
    }
}

impl ExecutionSpool {
    pub fn path(&self) -> &Path {
        self.path.as_deref().expect("live execution spool")
    }
    pub fn discard(mut self) -> io::Result<()> {
        self.cleanup()
    }
    /// Finish releasing this owned execution despite transient registry
    /// contention. Cleanup has its own caller-supplied bound: cancelling the
    /// calculation must not abandon resources before attempting their release.
    pub fn discard_with_wait(mut self, timeout: Duration) -> io::Result<()> {
        wait(timeout, &|| false, || self.cleanup())
    }
    fn cleanup(&mut self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let registry = self.root.registry()?;
        if let Some(lease) = self.lease.take() {
            lease.release()?;
        }
        let lease = FileLock::try_exclusive(platform::regular(&path.join(LEASE), false)?)?;
        lease.release()?;
        fs::remove_dir_all(path)?;
        self.path = None;
        platform::sync_directory(&self.root.path)?;
        registry.release()
    }
}

fn wait<T>(
    timeout: Duration,
    cancelled: &dyn Fn() -> bool,
    mut operation: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let start = Instant::now();
    loop {
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "execution spool acquisition cancelled",
            ));
        }
        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "execution spool contention deadline",
            ));
        }
        match operation() {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5).min(remaining))
            }
            result => return result,
        }
    }
}
impl Drop for ExecutionSpool {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn execution_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let parts: Vec<_> = name.split('-').collect();
    if parts.len() != 4 || parts[0] != "execution" {
        return false;
    }
    parts[1..].iter().all(|value| {
        value
            .parse::<u128>()
            .is_ok_and(|number| number.to_string() == *value)
    })
}

#[cfg(test)]
mod tests;
