use super::storage;
use crate::{StandardHost, db};
use mo_common::{ByteLength, digest};
use mo_operation_service::*;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest as _, Sha256};

impl StandardHost {
    pub fn begin_upload(
        &mut self,
        context: &CallContext,
        request: UploadRequest,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        context.require(Permission::WriteAssets)?;
        let descriptor = &request.descriptor;
        db::validate_media_type(&descriptor.media_type)?;
        let id = UploadId::new(format!(
            "upload:{}",
            digest(
                "musteroffice.upload-id/1",
                &(&context.scope, &context.principal, &request.request_id)
            )
            .map_err(|_| db::corrupt())?
        ))
        .map_err(|_| db::corrupt())?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM asset_uploads WHERE scope=?1 AND principal=?2 AND request_id=?3",
                params![
                    context.scope.as_str(),
                    context.principal.as_str(),
                    request.request_id.as_str()
                ],
                |r| r.get(0),
            )
            .optional()
            .map_err(db::error)?;
        if let Some(existing) = existing {
            if existing != id.as_str() {
                return Err(db::corrupt());
            }
            let mut info = storage::upload(&tx, context, &id)?;
            if info.descriptor != request.descriptor {
                return Err(Failure::new(
                    FailureCode::RequestIdReused,
                    "upload request id identifies different content",
                ));
            }
            storage::expire(&tx, context, &mut info, now)?;
            tx.commit().map_err(db::error)?;
            return Ok(info);
        }
        if descriptor.byte_length.get() > self.limits.assets.max_asset_bytes {
            return Err(Failure::new(FailureCode::LimitExceeded, "asset byte limit"));
        }
        storage::reap(&tx, &context.scope, now)?;
        crate::results::reap(&tx, &context.scope, now)?;
        let reserved = db::scope_reserved(&tx, &context.scope)?;
        let count: u32 = tx
            .query_row(
                "SELECT count(*) FROM asset_uploads WHERE scope=?1",
                [context.scope.as_str()],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        if count >= self.limits.assets.max_uploads_per_scope
            || reserved
                .checked_add(descriptor.byte_length.get())
                .is_none_or(|sum| sum > self.limits.assets.max_scope_bytes)
        {
            // Persist expiry cleanup even when this new admission is refused.
            tx.commit().map_err(db::error)?;
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "resource scope reservation or receipt quota",
            ));
        }
        let expiry = now
            .checked_add(self.limits.assets.upload_ttl_ms)
            .ok_or_else(|| Failure::new(FailureCode::LimitExceeded, "upload expiry overflow"))?;
        let info = UploadInfo {
            id,
            request_id: request.request_id,
            descriptor: request.descriptor,
            state: UploadState::Uploading,
            chunk_bytes: ASSET_CHUNK_BYTES as u32,
            received_bytes: ByteLength::new(0),
            created_at: now,
            updated_at: now,
            expires_at: Some(expiry),
            asset: None,
            error: None,
        };
        tx.execute("INSERT INTO asset_uploads(scope,principal,id,request_id,info,reserved_bytes,expires_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![context.scope.as_str(),context.principal.as_str(),info.id.as_str(),info.request_id.as_str(),db::encode(&info)?,info.descriptor.byte_length.get() as i64,expiry.get()]).map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    pub fn get_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        context.require(Permission::WriteAssets)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut info = storage::upload(&tx, context, id)?;
        storage::expire(&tx, context, &mut info, now)?;
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    /// Caller supplies one fixed-size binary chunk; final chunk may be shorter.
    /// offset is exact and resending the same already-accepted bytes is safe.
    pub fn append_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        offset: ByteLength,
        bytes: &[u8],
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        context.require(Permission::WriteAssets)?;
        if bytes.is_empty()
            || bytes.len() > ASSET_CHUNK_BYTES
            || !offset.get().is_multiple_of(ASSET_CHUNK_BYTES as u64)
        {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid upload chunk size or offset",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut info = storage::upload(&tx, context, id)?;
        storage::expire(&tx, context, &mut info, now)?;
        if matches!(info.state, UploadState::Failed | UploadState::Cancelled) {
            tx.commit().map_err(db::error)?;
            return Ok(info);
        }
        if info.state == UploadState::Verifying {
            return Err(Failure::new(
                FailureCode::ResourceBusy,
                "upload is frozen for verification",
            ));
        }
        let end = offset
            .get()
            .checked_add(bytes.len() as u64)
            .filter(|end| *end <= info.descriptor.byte_length.get())
            .ok_or_else(|| {
                Failure::new(
                    FailureCode::ResourceConflict,
                    "chunk exceeds declared resource",
                )
            })?;
        if bytes.len() != ASSET_CHUNK_BYTES && end != info.descriptor.byte_length.get() {
            return Err(Failure::new(
                FailureCode::ResourceConflict,
                "non-final chunk is incomplete",
            ));
        }
        let index = offset.get() / ASSET_CHUNK_BYTES as u64;
        if offset.get() < info.received_bytes.get() {
            let stored = storage::chunk(
                &tx,
                &context.scope,
                &context.principal,
                id,
                index,
                info.descriptor.byte_length,
            )?;
            if stored != bytes {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "chunk retry differs",
                ));
            }
            tx.commit().map_err(db::error)?;
            return Ok(info);
        }
        if offset != info.received_bytes {
            return Err(Failure::new(
                FailureCode::ResourceConflict,
                "chunk is not contiguous",
            ));
        }
        let expiry = now
            .checked_add(self.limits.assets.upload_ttl_ms)
            .ok_or_else(|| Failure::new(FailureCode::LimitExceeded, "upload expiry overflow"))?;
        tx.execute("INSERT INTO asset_chunks(scope,principal,upload_id,chunk_index,data,sha256) VALUES (?1,?2,?3,?4,?5,?6)",params![context.scope.as_str(),context.principal.as_str(),id.as_str(),index as i64,bytes,storage::hash(bytes).as_str()]).map_err(db::error)?;
        info.received_bytes = ByteLength::new(end);
        info.updated_at = now;
        info.expires_at = Some(expiry);
        storage::save(&tx, context, &info)?;
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    pub fn cancel_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        context.require(Permission::WriteAssets)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut info = storage::upload(&tx, context, id)?;
        storage::expire(&tx, context, &mut info, now)?;
        if !info.state.terminal() {
            storage::fail(
                &tx,
                context,
                &mut info,
                now,
                Failure::new(FailureCode::Cancelled, "resource upload cancelled"),
            )?;
        }
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    /// Explicit bounded maintenance; it can be repeated when more than 128
    /// uploads have expired. Completed assets/receipts are never deleted.
    pub fn reap_uploads(&mut self, context: &CallContext, now: UnixMillis) -> Result<u32, Failure> {
        context.require(Permission::WriteAssets)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let n = storage::reap(&tx, &context.scope, now)?;
        tx.commit().map_err(db::error)?;
        Ok(n)
    }
    /// Freeze first, verify actual persisted bytes without holding the write
    /// lock, then publish only if the same frozen upload is still eligible.
    pub fn seal_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<UploadInfo, Failure> {
        context.require(Permission::WriteAssets)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut info = storage::upload(&tx, context, id)?;
        let started = clock();
        storage::expire(&tx, context, &mut info, started)?;
        if info.state.terminal() {
            tx.commit().map_err(db::error)?;
            return Ok(info);
        }
        if info.state == UploadState::Verifying {
            return Err(Failure::new(
                FailureCode::ResourceBusy,
                "resource verification is already running",
            ));
        }
        if info.received_bytes != info.descriptor.byte_length {
            return Err(Failure::new(
                FailureCode::ResourceIncomplete,
                "resource upload is incomplete",
            ));
        }
        info.state = UploadState::Verifying;
        info.updated_at = started;
        info.expires_at = Some(
            started
                .checked_add(self.limits.assets.upload_ttl_ms)
                .ok_or_else(|| {
                    Failure::new(FailureCode::LimitExceeded, "upload expiry overflow")
                })?,
        );
        storage::save(&tx, context, &info)?;
        tx.commit().map_err(db::error)?;
        let verified = (|| {
            let mut hash = Sha256::new();
            let chunks = info
                .descriptor
                .byte_length
                .get()
                .div_ceil(ASSET_CHUNK_BYTES as u64);
            for index in 0..chunks {
                if check() {
                    return Err(Failure::new(
                        FailureCode::Cancelled,
                        "resource verification cancelled",
                    ));
                }
                let bytes = storage::chunk(
                    &self.connection,
                    &context.scope,
                    &context.principal,
                    id,
                    index,
                    info.descriptor.byte_length,
                )?;
                hash.update(&bytes);
            }
            if mo_common::Digest::from_sha256(hash.finalize().into()) != info.descriptor.sha256 {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "stored resource digest differs from declaration",
                ));
            }
            if check() {
                return Err(Failure::new(
                    FailureCode::Cancelled,
                    "resource verification cancelled",
                ));
            }
            Ok(())
        })();
        let now = clock();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut current = storage::upload(&tx, context, id)?;
        storage::expire(&tx, context, &mut current, now)?;
        if current.state.terminal() {
            tx.commit().map_err(db::error)?;
            return Ok(current);
        }
        if current != info {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "frozen upload changed during verification",
            ));
        }
        match verified {
            Err(error) => {
                // Storage faults may be transient; retain the frozen lease for
                // explicit observation/expiry instead of forging a terminal.
                if matches!(
                    error.code,
                    FailureCode::StorageFailure | FailureCode::StorageBusy
                ) {
                    return Err(error);
                }
                storage::fail(&tx, context, &mut current, now, error)?;
            }
            Ok(()) => {
                let asset = AssetInfo {
                    id: AssetId::new(format!(
                        "asset:{}",
                        digest(
                            "musteroffice.asset-id/1",
                            &(&context.scope, &context.principal, id, &info.descriptor)
                        )
                        .map_err(|_| db::corrupt())?
                    ))
                    .map_err(|_| db::corrupt())?,
                    descriptor: info.descriptor,
                    verification: AssetVerification::BytesSha256,
                };
                tx.execute(
                    "INSERT INTO assets(scope,id,principal,upload_id,info) VALUES (?1,?2,?3,?4,?5)",
                    params![
                        context.scope.as_str(),
                        asset.id.as_str(),
                        context.principal.as_str(),
                        id.as_str(),
                        db::encode(&asset)?
                    ],
                )
                .map_err(db::error)?;
                current.state = UploadState::Sealed;
                current.updated_at = now;
                current.expires_at = None;
                current.asset = Some(asset);
                storage::save(&tx, context, &current)?;
            }
        }
        tx.commit().map_err(db::error)?;
        Ok(current)
    }
}
