//! The exporter owns the private worker lifecycle. Once its synchronous
//! transport returns, all child access has stopped and this scope releases the
//! spool with a bounded contention wait, including cancelled/error unwinding.
use mo_native_io::ExecutionSpool;
use std::{io, path::Path, time::Duration};

pub(crate) struct RetainedSpool {
    spool: Option<ExecutionSpool>,
    timeout: Duration,
}
impl RetainedSpool {
    pub fn new(spool: ExecutionSpool, timeout: Duration) -> Self {
        Self {
            spool: Some(spool),
            timeout,
        }
    }
    pub fn path(&self) -> &Path {
        self.spool.as_ref().expect("live native execution").path()
    }
    pub fn discard(self) -> io::Result<()> {
        let timeout = self.timeout;
        self.discard_with_wait(timeout)
    }
    pub fn discard_with_wait(mut self, timeout: Duration) -> io::Result<()> {
        self.spool
            .take()
            .expect("live native execution")
            .discard_with_wait(timeout)
    }
}
impl Drop for RetainedSpool {
    fn drop(&mut self) {
        if let Some(spool) = self.spool.take() {
            let _ = spool.discard_with_wait(self.timeout);
        }
    }
}
