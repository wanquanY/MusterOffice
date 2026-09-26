use super::{Owner, Record, SqlResultReader, State, storage};
use crate::db;
use mo_common::Digest;
use mo_opc::{ReaderAt, ResultSink, SealedOutput};
use mo_operation_service::*;
use rusqlite::{Transaction, TransactionBehavior, params};
use sha2::{Digest as _, Sha256};
use std::io::{self, Write};

/// One bounded tail buffer. Flush persists a partial tail; further writes may
/// extend only that tail while Writing. After seal, all chunks are immutable.
/// Drop abandons unsealed bytes; crash recovery is owned by the job's lease.
pub struct SqlResultSink<'a> {
    owner: Owner<'a>,
    record: Record,
    buffer: Vec<u8>,
    accepted: u64,
    dirty: bool,
    failure: Option<Failure>,
    retained: bool,
}
impl<'a> SqlResultSink<'a> {
    pub(super) fn new(owner: Owner<'a>, record: Record) -> Self {
        Self {
            owner,
            record,
            buffer: Vec::new(),
            accepted: 0,
            dirty: false,
            failure: None,
            retained: false,
        }
    }
    fn healthy(&self) -> Result<(), Failure> {
        self.failure.clone().map_or(Ok(()), Err)
    }
    fn check(&self) -> Result<(), Failure> {
        self.healthy()?;
        let now = self.owner.now()?;
        let tx =
            Transaction::new_unchecked(&self.owner.host.connection, TransactionBehavior::Deferred)
                .map_err(db::error)?;
        storage::guard(&tx, &self.owner, now)?;
        let stored = storage::load(&tx, &self.owner)?;
        storage::check_record(&stored, State::Writing, now)?;
        self.check_persisted(&stored)?;
        tx.commit().map_err(db::error)
    }
    fn check_persisted(&self, stored: &Record) -> Result<(), Failure> {
        if stored.received != self.record.received || stored.spec != self.record.spec {
            return Err(db::corrupt());
        }
        Ok(())
    }
    fn persist(&mut self) -> Result<(), Failure> {
        self.healthy()?;
        if !self.dirty {
            return self.check();
        }
        let now = self.owner.now()?;
        let digest = storage::hash(&self.buffer);
        let tx =
            Transaction::new_unchecked(&self.owner.host.connection, TransactionBehavior::Immediate)
                .map_err(db::error)?;
        storage::guard(&tx, &self.owner, now)?;
        let mut stored = storage::load(&tx, &self.owner)?;
        storage::check_record(&stored, State::Writing, now)?;
        self.check_persisted(&stored)?;
        let base = self.accepted - self.buffer.len() as u64;
        if !base.is_multiple_of(ASSET_CHUNK_BYTES as u64)
            || stored.received < base
            || stored.received > self.accepted
            || self.accepted > stored.spec.max_bytes
        {
            return Err(db::corrupt());
        }
        let o = &self.owner;
        tx.execute(
            "INSERT INTO result_chunks(scope,principal,job_id,fence,name,chunk_index,data,sha256) VALUES (?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(scope,principal,job_id,fence,name,chunk_index) DO UPDATE SET data=excluded.data,sha256=excluded.sha256",
            params![o.context.scope.as_str(),o.context.principal.as_str(),o.lease.id.as_str(),o.lease.fence.get(),o.name.as_str(),(base / ASSET_CHUNK_BYTES as u64) as i64,&self.buffer,digest.as_str()],
        ).map_err(db::error)?;
        stored.received = self.accepted;
        stored.updated_at = now;
        storage::save(&tx, o, &stored)?;
        tx.commit().map_err(db::error)?;
        self.record = stored;
        self.dirty = false;
        if self.buffer.len() == ASSET_CHUNK_BYTES {
            self.buffer.clear();
        }
        Ok(())
    }
    fn append(&mut self, bytes: &[u8]) -> Result<usize, Failure> {
        self.check()?;
        // Preflight this write's full input before accepting any of it. A limit
        // failure poisons the capability, so a truncated output cannot seal.
        if self
            .accepted
            .checked_add(bytes.len() as u64)
            .is_none_or(|n| n > self.record.spec.max_bytes)
        {
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "result reserved byte limit",
            ));
        }
        if bytes.is_empty() {
            return Ok(0);
        }
        let n = bytes.len().min(ASSET_CHUNK_BYTES - self.buffer.len());
        self.buffer
            .try_reserve(n)
            .map_err(|_| Failure::new(FailureCode::LimitExceeded, "result buffer allocation"))?;
        self.buffer.extend_from_slice(&bytes[..n]);
        self.accepted += n as u64;
        self.dirty = true;
        if self.buffer.len() == ASSET_CHUNK_BYTES {
            self.persist()?;
        }
        Ok(n)
    }
    fn freeze(&mut self) -> Result<Record, Failure> {
        self.persist()?;
        let now = self.owner.now()?;
        let tx =
            Transaction::new_unchecked(&self.owner.host.connection, TransactionBehavior::Immediate)
                .map_err(db::error)?;
        storage::guard(&tx, &self.owner, now)?;
        let mut record = storage::load(&tx, &self.owner)?;
        storage::check_record(&record, State::Writing, now)?;
        self.check_persisted(&record)?;
        let o = &self.owner;
        let (count, total, last): (i64,i64,Option<i64>) = tx.query_row(
            "SELECT count(*),coalesce(sum(length(data)),0),max(chunk_index) FROM result_chunks WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",
            params![o.context.scope.as_str(),o.context.principal.as_str(),o.lease.id.as_str(),o.lease.fence.get(),o.name.as_str()],
            |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).map_err(db::error)?;
        let expected = record.received.div_ceil(ASSET_CHUNK_BYTES as u64);
        if total < 0
            || total as u64 != self.accepted
            || total as u64 != record.received
            || count != expected as i64
            || last != expected.checked_sub(1).map(|n| n as i64)
        {
            return Err(db::corrupt());
        }
        record.state = State::Sealing;
        record.updated_at = now;
        storage::save(&tx, o, &record)?;
        tx.commit().map_err(db::error)?;
        Ok(record)
    }
    fn finish_seal(&self, frozen: Record, digest: Digest) -> Result<Record, Failure> {
        let now = self.owner.now()?;
        let tx =
            Transaction::new_unchecked(&self.owner.host.connection, TransactionBehavior::Immediate)
                .map_err(db::error)?;
        storage::guard(&tx, &self.owner, now)?;
        let mut stored = storage::load(&tx, &self.owner)?;
        storage::check_record(&stored, State::Sealing, now)?;
        if stored.received != frozen.received || stored.spec != frozen.spec {
            return Err(db::corrupt());
        }
        stored.state = State::Sealed;
        stored.digest = Some(digest);
        stored.updated_at = now;
        storage::save(&tx, &self.owner, &stored)?;
        tx.commit().map_err(db::error)?;
        Ok(stored)
    }
    fn sticky<T>(&mut self, result: Result<T, Failure>) -> io::Result<T> {
        result.map_err(|failure| {
            self.failure = Some(failure.clone());
            io::Error::other(failure)
        })
    }
}
impl Write for SqlResultSink<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let result = self.append(bytes);
        self.sticky(result)
    }
    fn flush(&mut self) -> io::Result<()> {
        let result = self.persist();
        self.sticky(result)
    }
}
impl<'a> ResultSink for SqlResultSink<'a> {
    type Reader = SqlResultReader<'a>;
    fn seal(mut self) -> io::Result<SealedOutput<Self::Reader>> {
        let frozen = self.freeze().map_err(io::Error::other)?;
        // Free the tail before verifying persisted chunks; never retain the
        // writer's bytes as a substitute for reading final storage.
        self.buffer = Vec::new();
        let reader = SqlResultReader::new(self.owner.clone(), frozen.clone());
        let mut hash = Sha256::new();
        let mut offset = 0;
        let mut buffer = vec![0; ASSET_CHUNK_BYTES];
        while offset < frozen.received {
            let n = reader.read_at(&mut buffer, offset)?;
            if n == 0 {
                return Err(io::Error::other(db::corrupt()));
            }
            hash.update(&buffer[..n]);
            offset += n as u64;
        }
        let record = self
            .finish_seal(frozen, Digest::from_sha256(hash.finalize().into()))
            .map_err(io::Error::other)?;
        self.retained = true;
        Ok(SealedOutput {
            byte_length: record.received,
            reader: SqlResultReader::new(self.owner.clone(), record),
        })
    }
}
impl Drop for SqlResultSink<'_> {
    fn drop(&mut self) {
        if !self.retained
            && let Ok(tx) = Transaction::new_unchecked(
                &self.owner.host.connection,
                TransactionBehavior::Immediate,
            )
            && storage::abandon(&tx, &self.owner).is_ok()
        {
            let _ = tx.commit();
        }
    }
}
