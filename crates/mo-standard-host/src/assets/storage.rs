use crate::db;
use mo_common::{ByteLength, Digest};
use mo_operation_service::*;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest as _, Sha256};

pub(crate) fn migrate(c: &Connection) -> Result<(), Failure> {
    c.execute_batch("CREATE TABLE asset_uploads(scope TEXT NOT NULL, principal TEXT NOT NULL, id TEXT NOT NULL, request_id TEXT NOT NULL, info TEXT NOT NULL, reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes>=0), expires_at INTEGER, PRIMARY KEY(scope,principal,id), UNIQUE(scope,principal,request_id)) STRICT;
CREATE INDEX asset_upload_expiry ON asset_uploads(scope,expires_at);
CREATE TABLE asset_chunks(scope TEXT NOT NULL, principal TEXT NOT NULL, upload_id TEXT NOT NULL, chunk_index INTEGER NOT NULL CHECK(chunk_index>=0), data BLOB NOT NULL CHECK(length(data)>0 AND length(data)<=262144), sha256 TEXT NOT NULL, PRIMARY KEY(scope,principal,upload_id,chunk_index), FOREIGN KEY(scope,principal,upload_id) REFERENCES asset_uploads(scope,principal,id)) STRICT;
CREATE TABLE assets(scope TEXT NOT NULL, id TEXT NOT NULL, principal TEXT NOT NULL, upload_id TEXT NOT NULL, info TEXT NOT NULL, PRIMARY KEY(scope,id), UNIQUE(scope,principal,upload_id), FOREIGN KEY(scope,principal,upload_id) REFERENCES asset_uploads(scope,principal,id)) STRICT;").map_err(db::error)
}
pub(super) fn hash(bytes: &[u8]) -> Digest {
    Digest::from_sha256(Sha256::digest(bytes).into())
}
pub(super) fn upload(
    c: &Connection,
    context: &CallContext,
    id: &UploadId,
) -> Result<UploadInfo, Failure> {
    upload_for(c, &context.scope, &context.principal, id)
}
pub(super) fn upload_for(
    c: &Connection,
    scope: &ScopeId,
    principal: &PrincipalId,
    id: &UploadId,
) -> Result<UploadInfo, Failure> {
    let row:Option<(String,String,i64,Option<i64>)>=c.query_row("SELECT info,request_id,reserved_bytes,expires_at FROM asset_uploads WHERE scope=?1 AND principal=?2 AND id=?3",params![scope.as_str(),principal.as_str(),id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(db::error)?;
    let (json, request_id, reserved, expiry) = row.ok_or_else(db::not_found)?;
    let info: UploadInfo = db::decode(&json)?;
    let n = info.descriptor.byte_length.get();
    let error_state = matches!(info.state, UploadState::Cancelled | UploadState::Failed);
    if info.id != *id
        || info.request_id.as_str() != request_id
        || info.chunk_bytes as usize != ASSET_CHUNK_BYTES
        || info.received_bytes.get() > n
        || info.created_at > info.updated_at
        || expiry != info.expires_at.map(UnixMillis::get)
        || info.state.terminal() != info.expires_at.is_none()
        || error_state != info.error.is_some()
        || (info.state == UploadState::Sealed) != info.asset.is_some()
        || reserved < 0
        || reserved as u64 != if error_state { 0 } else { n }
    {
        return Err(db::corrupt());
    }
    if let Some(asset) = &info.asset
        && (asset.descriptor != info.descriptor
            || info.received_bytes != info.descriptor.byte_length)
    {
        return Err(db::corrupt());
    }
    if info.state == UploadState::Verifying && info.received_bytes != info.descriptor.byte_length {
        return Err(db::corrupt());
    }
    if info.error.as_ref().is_some_and(|e| {
        (e.code == FailureCode::Cancelled) != (info.state == UploadState::Cancelled)
    }) {
        return Err(db::corrupt());
    }
    Ok(info)
}
pub(super) fn save(
    c: &Connection,
    context: &CallContext,
    info: &UploadInfo,
) -> Result<(), Failure> {
    save_for(c, &context.scope, &context.principal, info)
}
fn save_for(
    c: &Connection,
    scope: &ScopeId,
    principal: &PrincipalId,
    info: &UploadInfo,
) -> Result<(), Failure> {
    let reserved = if matches!(info.state, UploadState::Failed | UploadState::Cancelled) {
        0
    } else {
        info.descriptor.byte_length.get()
    };
    let n=c.execute("UPDATE asset_uploads SET info=?1,reserved_bytes=?2,expires_at=?3 WHERE scope=?4 AND principal=?5 AND id=?6",params![db::encode(info)?,i64::try_from(reserved).map_err(|_|db::corrupt())?,info.expires_at.map(UnixMillis::get),scope.as_str(),principal.as_str(),info.id.as_str()]).map_err(db::error)?;
    if n != 1 {
        return Err(db::corrupt());
    }
    Ok(())
}
pub(super) fn fail(
    c: &Connection,
    context: &CallContext,
    info: &mut UploadInfo,
    now: UnixMillis,
    error: Failure,
) -> Result<(), Failure> {
    fail_for(c, &context.scope, &context.principal, info, now, error)
}
fn fail_for(
    c: &Connection,
    scope: &ScopeId,
    principal: &PrincipalId,
    info: &mut UploadInfo,
    now: UnixMillis,
    error: Failure,
) -> Result<(), Failure> {
    info.state = if error.code == FailureCode::Cancelled {
        UploadState::Cancelled
    } else {
        UploadState::Failed
    };
    info.updated_at = now;
    info.expires_at = None;
    info.asset = None;
    info.error = Some(error);
    c.execute(
        "DELETE FROM asset_chunks WHERE scope=?1 AND principal=?2 AND upload_id=?3",
        params![scope.as_str(), principal.as_str(), info.id.as_str()],
    )
    .map_err(db::error)?;
    save_for(c, scope, principal, info)
}
pub(super) fn expire(
    c: &Connection,
    context: &CallContext,
    info: &mut UploadInfo,
    now: UnixMillis,
) -> Result<(), Failure> {
    if info.state.terminal() {
        return Ok(());
    }
    if now < info.updated_at {
        return Err(Failure::new(
            FailureCode::InputInvalid,
            "host clock moved backwards",
        ));
    }
    if info.expires_at.is_some_and(|expiry| now >= expiry) {
        fail(
            c,
            context,
            info,
            now,
            Failure::new(FailureCode::ResourceExpired, "upload lease expired"),
        )?;
    }
    Ok(())
}
/// Bounded scope maintenance before quota admission. Does not reveal another
/// principal's upload metadata and never deletes a sealed asset.
pub(crate) fn reap(c: &Connection, scope: &ScopeId, now: UnixMillis) -> Result<u32, Failure> {
    let mut query=c.prepare("SELECT principal,id FROM asset_uploads WHERE scope=?1 AND expires_at<=?2 ORDER BY expires_at LIMIT 128").map_err(db::error)?;
    let rows: Vec<(String, String)> = query
        .query_map(params![scope.as_str(), now.get()], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(db::error)?
        .collect::<Result<_, _>>()
        .map_err(db::error)?;
    for (principal, id) in &rows {
        let principal = PrincipalId::new(principal.clone()).map_err(|_| db::corrupt())?;
        let id = UploadId::new(id.clone()).map_err(|_| db::corrupt())?;
        let mut info = upload_for(c, scope, &principal, &id)?;
        if info.state.terminal()
            || info.expires_at.is_none_or(|expiry| now < expiry)
            || now < info.updated_at
        {
            return Err(db::corrupt());
        }
        fail_for(
            c,
            scope,
            &principal,
            &mut info,
            now,
            Failure::new(FailureCode::ResourceExpired, "upload lease expired"),
        )?;
    }
    Ok(rows.len() as u32)
}
pub(super) fn asset(
    c: &Connection,
    scope: &ScopeId,
    id: &AssetId,
) -> Result<(AssetInfo, super::reader::Backing), Failure> {
    let row: Option<(String, String, String)> = c
        .query_row(
            "SELECT info,principal,upload_id FROM assets WHERE scope=?1 AND id=?2",
            params![scope.as_str(), id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(db::error)?;
    let Some((json, principal, upload_id)) = row else {
        return crate::results::published_asset(c, scope, id)
            .map(|(info, key)| (info, super::reader::Backing::Result(key)));
    };
    let info: AssetInfo = db::decode(&json)?;
    if info.id != *id {
        return Err(db::corrupt());
    }
    let principal = PrincipalId::new(principal).map_err(|_| db::corrupt())?;
    let upload_id = UploadId::new(upload_id).map_err(|_| db::corrupt())?;
    let upload = upload_for(c, scope, &principal, &upload_id)?;
    if upload.asset.as_ref() != Some(&info) || upload.state != UploadState::Sealed {
        return Err(db::corrupt());
    }
    Ok((
        info,
        super::reader::Backing::Upload {
            principal,
            upload: upload_id,
        },
    ))
}
pub(super) fn chunk(
    c: &Connection,
    scope: &ScopeId,
    principal: &PrincipalId,
    id: &UploadId,
    index: u64,
    total: ByteLength,
) -> Result<Vec<u8>, Failure> {
    let offset = index
        .checked_mul(ASSET_CHUNK_BYTES as u64)
        .ok_or_else(db::corrupt)?;
    let remaining = total
        .get()
        .checked_sub(offset)
        .filter(|n| *n > 0)
        .ok_or_else(db::corrupt)?;
    let expected = remaining.min(ASSET_CHUNK_BYTES as u64) as i64;
    // Check SQL length before copying a blob, even if the database is damaged.
    let (data,digest):(Option<Vec<u8>>,String)=c.query_row("SELECT CASE WHEN length(data)=?5 THEN data ELSE NULL END,sha256 FROM asset_chunks WHERE scope=?1 AND principal=?2 AND upload_id=?3 AND chunk_index=?4",params![scope.as_str(),principal.as_str(),id.as_str(),i64::try_from(index).map_err(|_|db::corrupt())?,expected],|r|Ok((r.get(0)?,r.get(1)?))).map_err(db::error)?;
    let data = data.ok_or_else(db::corrupt)?;
    if hash(&data).as_str() != digest {
        return Err(db::corrupt());
    }
    Ok(data)
}
