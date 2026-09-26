//! Publication reuses sealed chunks. No payload copy or synthetic upload.
use super::{Record, SqlResultReader, State, storage};
use crate::{WorkLease, db};
use mo_common::{ByteLength, RequestId};
use mo_operation_service::*;
use mo_presentation_delivery::ProducedArtifact;
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Clone)]
pub(crate) struct PublishedKey {
    principal: PrincipalId,
    job: JobId,
    fence: JobFence,
    name: RequestId,
}
pub(crate) fn migrate_published(c: &Connection) -> Result<(), Failure> {
    c.execute_batch("CREATE TABLE result_assets(scope TEXT NOT NULL, id TEXT NOT NULL, principal TEXT NOT NULL, job_id TEXT NOT NULL, fence INTEGER NOT NULL, name TEXT NOT NULL, info TEXT NOT NULL, PRIMARY KEY(scope,id), UNIQUE(scope,principal,job_id,fence,name), FOREIGN KEY(scope,principal,job_id,fence,name) REFERENCES result_spools(scope,principal,job_id,fence,name)) STRICT;").map_err(db::error)
}
pub(crate) fn published_asset(
    c: &Connection,
    scope: &ScopeId,
    id: &AssetId,
) -> Result<(AssetInfo, PublishedKey), Failure> {
    let row: Option<(String, String, String, i64, String)> = c
        .query_row(
            "SELECT info,principal,job_id,fence,name FROM result_assets WHERE scope=?1 AND id=?2",
            params![scope.as_str(), id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(db::error)?;
    let (json, principal, job, fence, name) = row.ok_or_else(db::not_found)?;
    let info: AssetInfo = db::decode(&json)?;
    let key = PublishedKey {
        principal: PrincipalId::new(principal).map_err(|_| db::corrupt())?,
        job: JobId::new(job).map_err(|_| db::corrupt())?,
        fence: JobFence::new(fence).map_err(|_| db::corrupt())?,
        name: RequestId::new(name).map_err(|_| db::corrupt())?,
    };
    let (json, reserved, expiry): (String,i64,Option<i64>) = c.query_row(
        "SELECT info,reserved_bytes,expires_at FROM result_spools WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",
        params![scope.as_str(),key.principal.as_str(),key.job.as_str(),key.fence.get(),key.name.as_str()],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).map_err(db::error)?;
    let record: Record = db::decode(&json)?;
    if info.id != *id
        || record.state != State::Published
        || record.spec.name != key.name
        || record.digest.as_ref() != Some(&info.descriptor.sha256)
        || record.spec.media_type != info.descriptor.media_type
        || record.received != info.descriptor.byte_length.get()
        || record.received > record.spec.max_bytes
        || reserved < 0
        || reserved as u64 != record.received
        || expiry.is_some()
    {
        return Err(db::corrupt());
    }
    Ok((info, key))
}
pub(crate) fn published_chunk(
    c: &Connection,
    scope: &ScopeId,
    key: &PublishedKey,
    index: u64,
    total: ByteLength,
) -> Result<Vec<u8>, Failure> {
    let expected = index
        .checked_mul(ASSET_CHUNK_BYTES as u64)
        .and_then(|offset| total.get().checked_sub(offset))
        .filter(|n| *n > 0)
        .ok_or_else(db::corrupt)?
        .min(ASSET_CHUNK_BYTES as u64);
    let (data,digest): (Option<Vec<u8>>,String) = c.query_row(
        "SELECT CASE WHEN length(data)=?7 THEN data ELSE NULL END,sha256 FROM result_chunks WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5 AND chunk_index=?6",
        params![scope.as_str(),key.principal.as_str(),key.job.as_str(),key.fence.get(),key.name.as_str(),index as i64,expected as i64],
        |r|Ok((r.get(0)?,r.get(1)?)),
    ).map_err(db::error)?;
    let data = data.ok_or_else(db::corrupt)?;
    if storage::hash(&data).as_str() != digest {
        return Err(db::corrupt());
    }
    Ok(data)
}

pub(crate) struct Publication<'a, 'h> {
    reader: &'a SqlResultReader<'h>,
    record: Record,
    info: AssetInfo,
    reuse: bool,
}
/// Complete all business/integrity checks before any public writes.
pub(crate) fn prepare_publication<'a, 'h>(
    c: &Connection,
    context: &CallContext,
    lease: &WorkLease,
    artifacts: &'a [ProducedArtifact<SqlResultReader<'h>>],
    now: UnixMillis,
) -> Result<Vec<Publication<'a, 'h>>, Failure> {
    let mut plan = Vec::new();
    for artifact in artifacts {
        let reader = artifact.reader();
        let o = &reader.owner;
        if o.context.scope != context.scope
            || o.context.principal != context.principal
            || o.lease.id != lease.id
            || o.lease.fence != lease.fence
            || o.lease.request_digest != lease.request_digest
            || o.lease.executor != lease.executor
            || o.name.as_str() != artifact.name()
        {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "export output execution differs",
            ));
        }
        let record = storage::load(c, o)?;
        storage::check_record(&record, State::Sealed, now)?;
        let a = artifact.asset();
        if record.digest.as_ref() != Some(&a.sha256)
            || record.received != a.byte_length.get()
            || record.spec.media_type != a.media_type
        {
            return Err(db::corrupt());
        }
        let (count,total,last):(i64,i64,Option<i64>)=c.query_row(
            "SELECT count(*),coalesce(sum(length(data)),0),max(chunk_index) FROM result_chunks WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5",
            params![context.scope.as_str(),context.principal.as_str(),lease.id.as_str(),lease.fence.get(),o.name.as_str()],
            |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).map_err(db::error)?;
        let chunks = record.received.div_ceil(ASSET_CHUNK_BYTES as u64);
        if count != chunks as i64
            || total < 0
            || total as u64 != record.received
            || last != chunks.checked_sub(1).map(|n| n as i64)
        {
            return Err(db::corrupt());
        }
        let info = AssetInfo {
            id: AssetId::new(a.id.as_str()).map_err(|_| db::corrupt())?,
            descriptor: AssetDescriptor {
                sha256: a.sha256.clone(),
                byte_length: a.byte_length,
                media_type: a.media_type.clone(),
            },
            verification: AssetVerification::BytesSha256,
        };
        let existing:Option<String>=c.query_row(
            "SELECT info FROM assets WHERE scope=?1 AND id=?2 UNION ALL SELECT info FROM result_assets WHERE scope=?1 AND id=?2",
            params![context.scope.as_str(),info.id.as_str()],|r|r.get(0),
        ).optional().map_err(db::error)?;
        let reuse = if let Some(existing) = existing {
            if db::decode::<AssetInfo>(&existing)? != info {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "published asset identity differs",
                ));
            }
            // Validate the immutable existing backing as well as the descriptor.
            let published: bool = c
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM result_assets WHERE scope=?1 AND id=?2)",
                    params![context.scope.as_str(), info.id.as_str()],
                    |r| r.get(0),
                )
                .map_err(db::error)?;
            if !published {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "export asset collides with upload identity",
                ));
            }
            published_asset(c, &context.scope, &info.id)?;
            true
        } else {
            false
        };
        plan.push(Publication {
            reader,
            record,
            info,
            reuse,
        });
    }
    Ok(plan)
}
/// Caller owns the same transaction as the successful job receipt. Any SQL
/// failure must roll back; duplicate candidates remain private for job cleanup.
pub(crate) fn commit_publication(
    c: &Connection,
    plan: Vec<Publication<'_, '_>>,
    now: UnixMillis,
) -> Result<(), Failure> {
    for mut item in plan {
        if item.reuse {
            continue;
        }
        let o = &item.reader.owner;
        item.record.state = State::Published;
        item.record.updated_at = now;
        storage::save(c, o, &item.record)?;
        c.execute("INSERT INTO result_assets(scope,id,principal,job_id,fence,name,info) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![o.context.scope.as_str(),item.info.id.as_str(),o.context.principal.as_str(),o.lease.id.as_str(),o.lease.fence.get(),o.name.as_str(),db::encode(&item.info)?]).map_err(db::error)?;
    }
    Ok(())
}
