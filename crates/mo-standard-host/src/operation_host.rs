//! SQLite implementation of the shared owner port. Routing lives in the pure
//! service; the existing storage, authorization and commit implementations stay
//! authoritative for both direct calls and transport dispatch.
use crate::{AssetReader, StandardHost};
use mo_common::{ByteLength, Digest, DocumentId};
use mo_operation_service::*;
use mo_presentation_delivery::DeliveryLimits;
use mo_presentation_edit::SnapshotRecord;

impl StandardHost {
    pub(crate) fn export_limits(&self) -> DeliveryLimits {
        let mut limits = DeliveryLimits::default();
        limits.max_asset_bytes = limits
            .max_asset_bytes
            .min(self.limits.assets.max_asset_bytes);
        limits.max_model_bytes = limits.max_model_bytes.min(limits.max_asset_bytes);
        limits.max_font_bytes = limits.max_font_bytes.min(limits.max_asset_bytes);
        limits.max_total_bytes = limits
            .max_total_bytes
            .min(self.limits.assets.max_scope_bytes);
        limits.max_artifacts = limits
            .max_artifacts
            .min(self.limits.results.max_outputs_per_job as usize);
        limits
    }
    pub fn capabilities(&self, context: &CallContext) -> HostCapabilities {
        let limits = self.export_limits();
        HostCapabilities::describe(
            context,
            self.executor.clone(),
            self.renderer.as_ref().map(|r| r.identity()),
            ServiceLimits {
                request_bytes: ByteLength::new(MAX_OPERATION_BYTES as u64),
                asset_chunk_bytes: ByteLength::new(ASSET_CHUNK_BYTES as u64),
                asset_bytes: ByteLength::new(self.limits.assets.max_asset_bytes),
                scope_reserved_bytes: ByteLength::new(self.limits.assets.max_scope_bytes),
                uploads_per_scope: self.limits.assets.max_uploads_per_scope,
                upload_ttl_ms: self.limits.assets.upload_ttl_ms as u32,
                jobs_per_principal: self.limits.max_jobs_per_principal,
                documents_per_scope: self.limits.max_documents_per_scope,
                lease_ms: self.limits.lease_ms as u32,
                outputs_per_job: self.limits.results.max_outputs_per_job,
                outputs_per_scope: self.limits.results.max_outputs_per_scope,
                export: ExportLimits {
                    pages: limits.max_pages as u32,
                    artifacts: limits.max_artifacts as u32,
                    model_bytes: ByteLength::new(limits.max_model_bytes),
                    asset_bytes: ByteLength::new(limits.max_asset_bytes),
                    font_bytes: ByteLength::new(limits.max_font_bytes),
                    total_bytes: ByteLength::new(limits.max_total_bytes),
                },
            },
            JobExecution::ExplicitRun,
        )
    }
}
impl AuthorizedAsset for AssetReader<'_> {
    fn info(&self) -> &AssetInfo {
        AssetReader::info(self)
    }
}
impl OperationHost for StandardHost {
    type Asset<'h> = AssetReader<'h>;
    fn capabilities(&self, context: &CallContext) -> HostCapabilities {
        StandardHost::capabilities(self, context)
    }
    fn begin_upload(
        &mut self,
        context: &CallContext,
        request: UploadRequest,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        StandardHost::begin_upload(self, context, request, now)
    }
    fn get_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        StandardHost::get_upload(self, context, id, now)
    }
    fn append_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        offset: ByteLength,
        bytes: &[u8],
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        StandardHost::append_upload(self, context, id, offset, bytes, now)
    }
    fn seal_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<UploadInfo, Failure> {
        StandardHost::seal_upload(self, context, id, clock, check)
    }
    fn cancel_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure> {
        StandardHost::cancel_upload(self, context, id, now)
    }
    fn asset_info(&self, context: &CallContext, id: &AssetId) -> Result<AssetInfo, Failure> {
        StandardHost::asset_info(self, context, id)
    }
    fn open_asset(&self, context: &CallContext, id: &AssetId) -> Result<Self::Asset<'_>, Failure> {
        StandardHost::open_asset(self, context, id)
    }
    fn submit(
        &mut self,
        context: &CallContext,
        request: OperationRequest,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        StandardHost::submit(self, context, request, now)
    }
    fn run_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<JobInfo, Failure> {
        StandardHost::run_job(self, context, id, now, clock, check)
    }
    fn get_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        StandardHost::get_job(self, context, id, now)
    }
    fn cancel_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        StandardHost::cancel_job(self, context, id, now)
    }
    fn read_document(
        &self,
        context: &CallContext,
        id: &DocumentId,
        revision: Option<&Digest>,
    ) -> Result<SnapshotRecord, Failure> {
        StandardHost::read_document(self, context, id, revision)
    }
}
