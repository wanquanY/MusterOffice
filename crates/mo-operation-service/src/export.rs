//! Legacy host adapter. Domain validation and export use the pure computation crate.
use crate::{AssetId, ExportAsset, ExportReceipt, Failure, FailureCode, OperationRequest};
use mo_common::Digest;
use mo_opc::ResultSink;
use mo_pptx::PptxError;
use mo_presentation_delivery::{
    DeliveryCandidate, DeliveryError, DeliveryLimits, OutputStore, PreviewRenderer,
};
use mo_presentation_edit::SnapshotRecord;
use mo_presentation_operations as computation;

pub trait ExportAssets {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure>;
}
struct Assets<'a>(&'a dyn ExportAssets);
impl computation::ExportAssets<Failure> for Assets<'_> {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure> {
        self.0.get(id)
    }
}
pub struct ExportCandidate<R> {
    request_digest: Digest,
    computed: computation::ExportCandidate<R>,
}
impl<R> ExportCandidate<R> {
    pub fn request_digest(&self) -> &Digest {
        &self.request_digest
    }
    pub fn delivery(&self) -> &DeliveryCandidate<R> {
        self.computed.delivery()
    }
    pub fn receipt(&self) -> ExportReceipt {
        self.computed.receipt()
    }
}
pub fn compute_export<S: OutputStore>(
    request: &OperationRequest,
    snapshot: SnapshotRecord,
    assets: &dyn ExportAssets,
    store: &mut S,
    renderer: &mut dyn PreviewRenderer,
    limits: DeliveryLimits,
    check: &dyn Fn() -> bool,
) -> Result<ExportCandidate<<S::Sink as ResultSink>::Reader>, Failure> {
    let computed = computation::compute_export::<S, Failure>(
        &crate::compatibility::input(request),
        snapshot,
        &Assets(assets),
        store,
        renderer,
        limits,
        check,
    )?;
    let request_digest = request
        .digest()
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "canonical export request"))?;
    if check() {
        return Err(Failure::new(FailureCode::Cancelled, "export cancelled"));
    }
    Ok(ExportCandidate {
        request_digest,
        computed,
    })
}

pub fn delivery_failure(error: DeliveryError) -> Failure {
    // Transparent error sources may delegate past the io::Error itself.
    let storage = match &error {
        DeliveryError::Io(e)
        | DeliveryError::Png(mo_image::png::PngError::Io(e))
        | DeliveryError::Pptx(PptxError::Opc(mo_opc::OpcError::Io(e))) => Some(e),
        _ => None,
    };
    if let Some(io) = storage {
        if let Some(f) = io.get_ref().and_then(|e| e.downcast_ref::<Failure>()) {
            return f.clone();
        }
        return Failure::new(
            FailureCode::StorageFailure,
            "export storage operation failed",
        );
    }
    // Preserve storage authority errors through OPC/PNG/io wrappers.
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(current) = cause {
        if let Some(f) = current.downcast_ref::<Failure>() {
            return f.clone();
        }
        if let Some(io) = current.downcast_ref::<std::io::Error>() {
            if let Some(f) = io.get_ref().and_then(|e| e.downcast_ref::<Failure>()) {
                return f.clone();
            }
            return Failure::new(
                FailureCode::StorageFailure,
                "export storage operation failed",
            );
        }
        cause = current.source();
    }
    computation::delivery_failure(error).into()
}
