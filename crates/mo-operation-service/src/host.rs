//! Shared routing, independent of persistence or transport. The injected host
//! remains the only authority for jobs, resource lifetimes and atomic commits.
use crate::*;
use mo_common::{ByteLength, Digest, DocumentId};
use mo_opc::ReaderAt;
use mo_presentation_edit::SnapshotRecord;

pub trait AuthorizedAsset: ReaderAt {
    fn info(&self) -> &AssetInfo;
}
/// Synchronous owner port, suitable inside a dedicated host worker. It neither
/// creates an owner nor schedules background work. Browser/async runtimes must
/// provide their execution and storage bridge, not call SQLite or block UI.
pub trait OperationHost {
    type Asset<'h>: AuthorizedAsset
    where
        Self: 'h;
    fn capabilities(&self, context: &CallContext) -> HostCapabilities;
    fn begin_upload(
        &mut self,
        context: &CallContext,
        request: UploadRequest,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure>;
    fn get_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure>;
    fn append_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        offset: ByteLength,
        bytes: &[u8],
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure>;
    fn seal_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<UploadInfo, Failure>;
    fn cancel_upload(
        &mut self,
        context: &CallContext,
        id: &UploadId,
        now: UnixMillis,
    ) -> Result<UploadInfo, Failure>;
    fn asset_info(&self, context: &CallContext, id: &AssetId) -> Result<AssetInfo, Failure>;
    fn open_asset(&self, context: &CallContext, id: &AssetId) -> Result<Self::Asset<'_>, Failure>;
    fn submit(
        &mut self,
        context: &CallContext,
        request: OperationRequest,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure>;
    fn run_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<JobInfo, Failure>;
    fn get_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure>;
    fn cancel_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure>;
    fn read_document(
        &self,
        context: &CallContext,
        id: &DocumentId,
        revision: Option<&Digest>,
    ) -> Result<SnapshotRecord, Failure>;
}
impl HostRequest {
    pub fn service_operation(&self) -> Result<ServiceOperation, Failure> {
        Ok(match self {
            Self::Capabilities {} => ServiceOperation::Capabilities,
            Self::GetSchema { .. } => ServiceOperation::Schema,
            Self::BeginUpload { .. } => ServiceOperation::BeginUpload,
            Self::GetUpload { .. } => ServiceOperation::GetUpload,
            Self::SealUpload { .. } => ServiceOperation::SealUpload,
            Self::CancelUpload { .. } => ServiceOperation::CancelUpload,
            Self::ReadAsset { .. } => ServiceOperation::ReadAsset,
            Self::Submit { request } => return ServiceOperation::for_action(&request.action),
            Self::GetJob { .. } => ServiceOperation::GetJob,
            Self::CancelJob { .. } => ServiceOperation::CancelJob,
            Self::ReadDocument { .. } => ServiceOperation::ReadDocument,
        })
    }
}
pub fn dispatch_host<H: OperationHost>(
    host: &mut H,
    context: &CallContext,
    request: HostRequest,
    clock: &dyn Fn() -> UnixMillis,
    check: &dyn Fn() -> bool,
) -> HostResponse {
    let result = (|| {
        // A backend must enforce authority on direct port calls too. Dispatch
        // checks before invoking it, so permission denial has no side effects.
        if let HostRequest::Submit { request } = &request {
            context.authorize(request)?;
        } else {
            request.service_operation()?.authorize(context)?;
        }
        match request {
            HostRequest::Capabilities {} => Ok(HostResponse::Succeeded {
                result: HostResult::Capabilities {
                    capabilities: Box::new(host.capabilities(context)),
                },
            }),
            HostRequest::GetSchema { id } => {
                id.document().map(|document| HostResponse::Succeeded {
                    result: HostResult::Schema {
                        document: Box::new(document),
                    },
                })
            }
            HostRequest::BeginUpload { request } => host
                .begin_upload(context, request, clock())
                .map(HostResponse::upload),
            HostRequest::GetUpload { upload_id } => host
                .get_upload(context, &upload_id, clock())
                .map(|upload| HostResponse::Succeeded {
                    result: HostResult::Upload {
                        upload: Box::new(upload),
                    },
                }),
            HostRequest::SealUpload { upload_id } => host
                .seal_upload(context, &upload_id, clock, check)
                .map(HostResponse::upload),
            HostRequest::CancelUpload { upload_id } => host
                .cancel_upload(context, &upload_id, clock())
                .map(HostResponse::upload),
            HostRequest::ReadAsset { asset_id } => {
                host.asset_info(context, &asset_id)
                    .map(|asset| HostResponse::Succeeded {
                        result: HostResult::Asset { asset },
                    })
            }
            HostRequest::Submit { request } => {
                let mode = request.output_mode;
                let job = host.submit(context, *request, clock())?;
                let job = if mode != OutputMode::Job && !job.state.terminal() {
                    host.run_job(context, &job.id, clock(), clock, check)?
                } else {
                    job
                };
                Ok(HostResponse::job(job))
            }
            HostRequest::GetJob { job_id } => host
                .get_job(context, &job_id, clock())
                .map(HostResponse::job),
            HostRequest::CancelJob { job_id } => host
                .cancel_job(context, &job_id, clock())
                .map(HostResponse::job),
            HostRequest::ReadDocument {
                document_id,
                revision,
            } => host
                .read_document(context, &document_id, revision.as_ref())
                .map(|snapshot| HostResponse::Succeeded {
                    result: HostResult::Document {
                        snapshot: Box::new(snapshot),
                    },
                }),
        }
    })();
    result.unwrap_or_else(|error| HostResponse::Failed { error, job: None })
}
pub fn dispatch_host_json<H: OperationHost>(
    host: &mut H,
    context: &CallContext,
    input: &str,
    clock: &dyn Fn() -> UnixMillis,
    check: &dyn Fn() -> bool,
) -> String {
    let request = decode_host_request(input);
    let response = match request {
        Ok(request) => dispatch_host(host, context, request, clock, check),
        Err(error) => HostResponse::Failed { error, job: None },
    };
    serde_json::to_string(&response).expect("typed host response")
}
pub fn decode_host_request(input: &str) -> Result<HostRequest, Failure> {
    if input.len() > MAX_OPERATION_BYTES {
        Err(Failure::new(
            FailureCode::LimitExceeded,
            "operation request bytes",
        ))
    } else {
        mo_common::from_json_str(input)
            .map_err(|_| Failure::new(FailureCode::InputInvalid, "invalid operation request"))
    }
}
