//! A lock belongs to its protocol scope, not to the last duplicated file
//! descriptor. In particular, an unrelated concurrent fork must not extend it.
use std::{
    fs::{File, TryLockError},
    io,
};

#[derive(Debug)]
pub(super) struct FileLock {
    file: File,
    locked: bool,
}

impl FileLock {
    pub(super) fn try_exclusive(file: File) -> Result<Self, TryLockError> {
        file.try_lock()?;
        Ok(Self { file, locked: true })
    }

    pub(super) fn try_shared(file: File) -> io::Result<Self> {
        file.try_lock_shared()?;
        Ok(Self { file, locked: true })
    }

    pub(super) fn shared(file: File) -> io::Result<Self> {
        file.lock_shared()?;
        Ok(Self { file, locked: true })
    }

    pub(super) fn sync_all(&self) -> io::Result<()> {
        self.file.sync_all()
    }

    pub(super) fn release(mut self) -> io::Result<()> {
        self.file.unlock()?;
        self.locked = false;
        Ok(())
    }

    #[cfg(all(test, unix))]
    pub(super) fn duplicate_descriptor(&self) -> io::Result<File> {
        self.file.try_clone()
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if self.locked {
            // Error paths cannot return cleanup errors; normal fallible paths
            // call release explicitly and propagate an unlock failure.
            let _ = self.file.unlock();
        }
    }
}
