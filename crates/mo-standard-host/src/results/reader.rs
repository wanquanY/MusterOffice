use super::{Owner, Record, storage};
use crate::db;
use mo_common::Digest;
use mo_opc::ReaderAt;
use mo_operation_service::*;
use rusqlite::{Transaction, TransactionBehavior};
use std::io;

/// Same persisted bytes, still private to a live execution. No whole-file or
/// chunk cache survives a read. Cancellation/expiry invalidates even existing
/// readers. A short read transaction makes the lease and bytes one snapshot.
pub struct SqlResultReader<'a> {
    pub(super) owner: Owner<'a>,
    pub(super) record: Record,
}
impl<'a> SqlResultReader<'a> {
    pub(super) fn new(owner: Owner<'a>, record: Record) -> Self {
        Self { owner, record }
    }
    pub fn byte_length(&self) -> u64 {
        self.record.received
    }
    pub fn sha256(&self) -> &Digest {
        self.record
            .digest
            .as_ref()
            .expect("public readers are sealed")
    }
    fn read(&self, buf: &mut [u8], offset: u64) -> Result<usize, Failure> {
        let now = self.owner.now()?;
        let tx =
            Transaction::new_unchecked(&self.owner.host.connection, TransactionBehavior::Deferred)
                .map_err(db::error)?;
        storage::guard(&tx, &self.owner, now)?;
        let stored = storage::load(&tx, &self.owner)?;
        storage::check_record(&stored, self.record.state, now)?;
        if stored.received != self.record.received
            || stored.digest != self.record.digest
            || stored.spec != self.record.spec
        {
            return Err(db::corrupt());
        }
        let n = if buf.is_empty() || offset >= stored.received {
            0
        } else {
            let index = offset / ASSET_CHUNK_BYTES as u64;
            let chunk = storage::chunk(&tx, &self.owner, index, stored.received)?;
            let start = (offset % ASSET_CHUNK_BYTES as u64) as usize;
            let n = buf.len().min(chunk.len() - start);
            buf[..n].copy_from_slice(&chunk[start..start + n]);
            n
        };
        tx.commit().map_err(db::error)?;
        Ok(n)
    }
}
impl ReaderAt for SqlResultReader<'_> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        self.read(buf, offset).map_err(io::Error::other)
    }
}
