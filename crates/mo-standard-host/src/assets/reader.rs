use super::storage;
use crate::StandardHost;
use mo_opc::ReaderAt;
use mo_operation_service::*;
use rusqlite::Connection;
use std::io;

/// Authorized immutable bytes, borrowing the host's connection. No chunk or
/// whole-file cache is retained. Scope permission is checked at open time;
/// transports must authorize every newly opened read/range request.
pub struct AssetReader<'a> {
    connection: &'a Connection,
    scope: ScopeId,
    backing: Backing,
    info: AssetInfo,
}
pub(super) enum Backing {
    Upload {
        principal: PrincipalId,
        upload: UploadId,
    },
    Result(crate::results::PublishedKey),
}
impl AssetReader<'_> {
    pub fn info(&self) -> &AssetInfo {
        &self.info
    }
}
impl ReaderAt for AssetReader<'_> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        if buf.is_empty() || offset >= self.info.descriptor.byte_length.get() {
            return Ok(0);
        }
        let index = offset / ASSET_CHUNK_BYTES as u64;
        let chunk = match &self.backing {
            Backing::Upload { principal, upload } => storage::chunk(
                self.connection,
                &self.scope,
                principal,
                upload,
                index,
                self.info.descriptor.byte_length,
            ),
            Backing::Result(key) => crate::results::published_chunk(
                self.connection,
                &self.scope,
                key,
                index,
                self.info.descriptor.byte_length,
            ),
        }
        .map_err(io::Error::other)?;
        let start = (offset % ASSET_CHUNK_BYTES as u64) as usize;
        let n = buf.len().min(chunk.len() - start);
        buf[..n].copy_from_slice(&chunk[start..start + n]);
        Ok(n)
    }
}
impl StandardHost {
    pub fn asset_info(&self, context: &CallContext, id: &AssetId) -> Result<AssetInfo, Failure> {
        context.require(Permission::ReadAssets)?;
        storage::asset(&self.connection, &context.scope, id).map(|(info, _)| info)
    }
    pub fn open_asset(
        &self,
        context: &CallContext,
        id: &AssetId,
    ) -> Result<AssetReader<'_>, Failure> {
        context.require(Permission::ReadAssets)?;
        let (info, backing) = storage::asset(&self.connection, &context.scope, id)?;
        Ok(AssetReader {
            connection: &self.connection,
            scope: context.scope.clone(),
            backing,
            info,
        })
    }
}
