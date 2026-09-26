use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

/// An execution-private directory under a protected operator-owned root.
/// The owner must reserve aggregate disk space and track this execution for
/// recovery after its own process crashes. It is never an Artifact publication.
pub struct SpoolDirectory {
    path: Option<PathBuf>,
}
impl SpoolDirectory {
    pub fn create(operator_root: &Path) -> io::Result<Self> {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for _ in 0..128 {
            let nonce = super::NEXT_SPOOL.fetch_add(1, Ordering::Relaxed);
            let path = operator_root.join(format!(
                "mo-execution-{}-{epoch}-{nonce}",
                std::process::id()
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path: Some(path) }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::other(
            "cannot allocate execution spool directory",
        ))
    }
    /// Trusted host configuration only; this path never enters document JSON.
    pub fn path(&self) -> &Path {
        self.path.as_deref().expect("live execution directory")
    }
    /// Call after workers are reaped and retained file readers are closed.
    /// Includes abandoned child spools left by a killed worker.
    pub fn discard(mut self) -> io::Result<()> {
        self.cleanup()
    }
    fn cleanup(&mut self) -> io::Result<()> {
        if let Some(path) = &self.path {
            remove(path)?;
            self.path = None;
        }
        Ok(())
    }
}
fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
impl Drop for SpoolDirectory {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_cleanup_relinquishes_path_before_drop() {
        let mut directory = SpoolDirectory::create(&std::env::temp_dir()).unwrap();
        let path = directory.path().to_path_buf();
        directory.cleanup().unwrap();
        fs::create_dir(&path).unwrap();
        let marker = path.join("new-owner");
        fs::write(&marker, b"keep").unwrap();
        drop(directory);
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
        fs::remove_dir_all(path).unwrap();
    }
}
