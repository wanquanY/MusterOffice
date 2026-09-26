//! Persistent binary resources. Short chunk transactions do not hold a database
//! write lock while waiting on client I/O or hashing an entire resource.
mod bindings;
mod reader;
pub use bindings::BoundResources;
mod storage;
mod upload;
use mo_operation_service::{Failure, FailureCode};
pub use reader::AssetReader;
pub(crate) use storage::migrate;
pub(crate) use storage::reap;

#[derive(Debug, Clone, Copy)]
pub struct AssetLimits {
    pub max_asset_bytes: u64,
    pub max_scope_bytes: u64,
    pub max_uploads_per_scope: u32,
    pub upload_ttl_ms: i64,
}
impl Default for AssetLimits {
    fn default() -> Self {
        Self {
            max_asset_bytes: 1024 * 1024 * 1024,
            max_scope_bytes: 4 * 1024 * 1024 * 1024,
            max_uploads_per_scope: 10_000,
            upload_ttl_ms: 900_000,
        }
    }
}
impl AssetLimits {
    pub(crate) fn validate(&self) -> Result<(), Failure> {
        if self.max_asset_bytes == 0
            || self.max_asset_bytes > self.max_scope_bytes
            || self.max_scope_bytes > i64::MAX as u64
            || self.max_uploads_per_scope == 0
            || !(1..=86_400_000).contains(&self.upload_ttl_ms)
        {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid asset limits",
            ));
        }
        Ok(())
    }
}
