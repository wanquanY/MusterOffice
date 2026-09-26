use super::{Owner, Record, State};
use crate::{db, execution::guard_job};
use mo_common::Digest;
use mo_operation_service::*;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest as _, Sha256};

pub(crate) fn migrate(c: &Connection) -> Result<(), Failure> {
    c.execute_batch("CREATE TABLE result_spools(scope TEXT NOT NULL, principal TEXT NOT NULL, job_id TEXT NOT NULL, fence INTEGER NOT NULL CHECK(fence>0), name TEXT NOT NULL, request_digest TEXT NOT NULL, executor_digest TEXT NOT NULL, info TEXT NOT NULL, reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes>=0), expires_at INTEGER, PRIMARY KEY(scope,principal,job_id,fence,name), FOREIGN KEY(scope,principal,job_id) REFERENCES jobs(scope,principal,id)) STRICT;
CREATE INDEX result_expiry ON result_spools(scope,expires_at);
CREATE TABLE result_chunks(scope TEXT NOT NULL, principal TEXT NOT NULL, job_id TEXT NOT NULL, fence INTEGER NOT NULL, name TEXT NOT NULL, chunk_index INTEGER NOT NULL CHECK(chunk_index>=0), data BLOB NOT NULL CHECK(length(data)>0 AND length(data)<=262144), sha256 TEXT NOT NULL, PRIMARY KEY(scope,principal,job_id,fence,name,chunk_index), FOREIGN KEY(scope,principal,job_id,fence,name) REFERENCES result_spools(scope,principal,job_id,fence,name)) STRICT;").map_err(db::error)
}

pub(super) fn hash(bytes: &[u8]) -> Digest {
    Digest::from_sha256(Sha256::digest(bytes).into())
}
/// The immutable request was validated when the capability was constructed.
/// Do not decode a potentially large document on every bounded stream access.
pub(super) fn guard(c: &Connection, owner: &Owner<'_>, now: UnixMillis) -> Result<(), Failure> {
    let info: Option<String> = c.query_row(
        "SELECT CASE WHEN length(info)<=65536 THEN info ELSE NULL END FROM jobs WHERE scope=?1 AND principal=?2 AND id=?3",
        params![owner.context.scope.as_str(), owner.context.principal.as_str(), owner.lease.id.as_str()],
        |r| r.get(0),
    ).map_err(db::error)?;
    let info: JobInfo = db::decode(&info.ok_or_else(db::corrupt)?)?;
    guard_job(&info, &owner.lease, now)
}
struct StoredRecord {
    json: Option<String>,
    request: String,
    executor: String,
    reserved: i64,
    expiry: Option<i64>,
}
pub(super) fn load(c: &Connection, owner: &Owner<'_>) -> Result<Record, Failure> {
    let row: Option<StoredRecord> = c.query_row(
        "SELECT CASE WHEN length(info)<=16384 THEN info ELSE NULL END,request_digest,executor_digest,reserved_bytes,expires_at FROM result_spools WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",
        params![owner.context.scope.as_str(), owner.context.principal.as_str(), owner.lease.id.as_str(), owner.lease.fence.get(), owner.name.as_str()],
        |r| Ok(StoredRecord { json:r.get(0)?,request:r.get(1)?,executor:r.get(2)?,reserved:r.get(3)?,expiry:r.get(4)? }),
    ).optional().map_err(db::error)?;
    let StoredRecord {
        json,
        request,
        executor,
        reserved,
        expiry,
    } = row.ok_or_else(db::not_found)?;
    let record: Record = db::decode(&json.ok_or_else(db::corrupt)?)?;
    let expected = match record.state {
        State::Writing | State::Sealing => record.spec.max_bytes,
        State::Sealed | State::Published => record.received,
        State::Discarded => 0,
    };
    if record.spec.name != owner.name
        || record.received > record.spec.max_bytes
        || request != owner.lease.request_digest.as_str()
        || executor != owner.lease.executor.as_str()
        || reserved < 0
        || reserved as u64 != expected
        || matches!(record.state, State::Sealed | State::Published) != record.digest.is_some()
        || matches!(record.state, State::Discarded | State::Published) != expiry.is_none()
    {
        return Err(db::corrupt());
    }
    Ok(record)
}
pub(super) fn check_record(record: &Record, state: State, now: UnixMillis) -> Result<(), Failure> {
    if record.state != state {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "result storage state differs",
        ));
    }
    if now < record.updated_at {
        return Err(Failure::new(
            FailureCode::InputInvalid,
            "result clock moved backwards",
        ));
    }
    Ok(())
}
pub(super) fn save(c: &Connection, owner: &Owner<'_>, record: &Record) -> Result<(), Failure> {
    let reserved = match record.state {
        State::Sealed | State::Published => record.received,
        State::Discarded => 0,
        _ => record.spec.max_bytes,
    };
    let n = c.execute(
        "UPDATE result_spools SET info=?6,reserved_bytes=?7,expires_at=CASE WHEN ?8 THEN NULL ELSE expires_at END WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",
        params![owner.context.scope.as_str(), owner.context.principal.as_str(), owner.lease.id.as_str(), owner.lease.fence.get(), owner.name.as_str(), db::encode(record)?, reserved as i64, matches!(record.state, State::Discarded | State::Published)],
    ).map_err(db::error)?;
    if n != 1 {
        return Err(db::corrupt());
    }
    Ok(())
}

fn discard(
    c: &Connection,
    scope: &str,
    principal: &str,
    job: &str,
    fence: i64,
    name: &str,
) -> Result<(), Failure> {
    // Persist a tombstone so an abandoned name cannot silently identify new bytes.
    let json: String = c.query_row("SELECT info FROM result_spools WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5", params![scope,principal,job,fence,name], |r|r.get(0)).map_err(db::error)?;
    let mut record: Record = db::decode(&json)?;
    if record.state == State::Published {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "published result cannot be discarded",
        ));
    }
    record.state = State::Discarded;
    record.digest = None;
    c.execute("DELETE FROM result_chunks WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",params![scope,principal,job,fence,name]).map_err(db::error)?;
    c.execute("UPDATE result_spools SET info=?6,reserved_bytes=0,expires_at=NULL WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",params![scope,principal,job,fence,name,db::encode(&record)?]).map_err(db::error)?;
    Ok(())
}
pub(super) fn abandon(c: &Connection, owner: &Owner<'_>) -> Result<(), Failure> {
    discard(
        c,
        owner.context.scope.as_str(),
        owner.context.principal.as_str(),
        owner.lease.id.as_str(),
        owner.lease.fence.get(),
        owner.name.as_str(),
    )
}

/// Called inside the same transaction as the job state change. Mutation jobs
/// have no published outputs: all their temporary results must be discarded.
pub(crate) fn job_changed(
    c: &Connection,
    context: &CallContext,
    job: &JobInfo,
) -> Result<(), Failure> {
    if job.state.terminal() || job.cancel_requested {
        let mut query = c.prepare("SELECT fence,name FROM result_spools WHERE scope=?1 AND principal=?2 AND job_id=?3 AND expires_at IS NOT NULL").map_err(db::error)?;
        let rows: Vec<(i64, String)> = query
            .query_map(
                params![
                    context.scope.as_str(),
                    context.principal.as_str(),
                    job.id.as_str()
                ],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db::error)?
            .collect::<Result<_, _>>()
            .map_err(db::error)?;
        for (fence, name) in rows {
            discard(
                c,
                context.scope.as_str(),
                context.principal.as_str(),
                job.id.as_str(),
                fence,
                &name,
            )?;
        }
    } else if let Some(until) = job.lease_until {
        c.execute("UPDATE result_spools SET expires_at=?4 WHERE scope=?1 AND principal=?2 AND job_id=?3 AND expires_at IS NOT NULL",params![context.scope.as_str(),context.principal.as_str(),job.id.as_str(),until.get()]).map_err(db::error)?;
    }
    Ok(())
}
pub(crate) fn reap(c: &Connection, scope: &ScopeId, now: UnixMillis) -> Result<u32, Failure> {
    let mut query = c.prepare("SELECT principal,job_id,fence,name FROM result_spools WHERE scope=?1 AND expires_at<=?2 ORDER BY expires_at LIMIT 128").map_err(db::error)?;
    let rows: Vec<(String, String, i64, String)> = query
        .query_map(params![scope.as_str(), now.get()], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(db::error)?
        .collect::<Result<_, _>>()
        .map_err(db::error)?;
    for (principal, job, fence, name) in &rows {
        discard(c, scope.as_str(), principal, job, *fence, name)?;
    }
    Ok(rows.len() as u32)
}

pub(super) fn chunk(
    c: &Connection,
    owner: &Owner<'_>,
    index: u64,
    total: u64,
) -> Result<Vec<u8>, Failure> {
    let remaining = index
        .checked_mul(ASSET_CHUNK_BYTES as u64)
        .and_then(|offset| total.checked_sub(offset))
        .filter(|n| *n > 0)
        .ok_or_else(db::corrupt)?;
    let expected = remaining.min(ASSET_CHUNK_BYTES as u64) as i64;
    let (data,digest): (Option<Vec<u8>>, String) = c.query_row(
        "SELECT CASE WHEN length(data)=?7 THEN data ELSE NULL END,sha256 FROM result_chunks WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5 AND chunk_index=?6",
        params![owner.context.scope.as_str(),owner.context.principal.as_str(),owner.lease.id.as_str(),owner.lease.fence.get(),owner.name.as_str(),index as i64,expected],
        |r|Ok((r.get(0)?,r.get(1)?)),
    ).map_err(db::error)?;
    let data = data.ok_or_else(db::corrupt)?;
    if hash(&data).as_str() != digest {
        return Err(db::corrupt());
    }
    Ok(data)
}
