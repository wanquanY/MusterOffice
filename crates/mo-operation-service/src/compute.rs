//! Legacy request/receipt adapter to the single presentation computation path.
use crate::{ExportAsset, Failure, FailureCode, MutationReceipt, OperationRequest};
use mo_common::{Digest, DocumentId};
use mo_presentation_edit::SnapshotRecord;
use mo_presentation_operations as computation;

pub struct MutationCandidate {
    request_digest: Digest,
    computed: computation::MutationCandidate,
    snapshot_json: String,
}
impl MutationCandidate {
    pub fn request_digest(&self) -> &Digest {
        &self.request_digest
    }
    pub fn document_id(&self) -> &DocumentId {
        self.computed.document_id()
    }
    pub fn base_revision(&self) -> Option<&Digest> {
        self.computed.base_revision()
    }
    pub fn base_semantic_digest(&self) -> Option<&Digest> {
        self.computed.base_semantic_digest()
    }
    pub fn snapshot(&self) -> &SnapshotRecord {
        self.computed.snapshot()
    }
    pub fn snapshot_json(&self) -> &str {
        &self.snapshot_json
    }
    pub fn receipt(&self) -> &MutationReceipt {
        self.computed.receipt()
    }
}
fn bind(
    request: &OperationRequest,
    computed: computation::MutationCandidate,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    let request_digest = request
        .digest()
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "canonical operation request"))?;
    let snapshot_json = serde_json::to_string(computed.snapshot())
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "snapshot serialization"))?;
    // The legacy envelope performs work after pure computation has returned.
    // Preserve cancellation at the end of that additional materialization.
    if check() {
        return Err(Failure::new(FailureCode::Cancelled, "operation cancelled"));
    }
    Ok(MutationCandidate {
        request_digest,
        computed,
        snapshot_json,
    })
}
pub fn compute_mutation(
    request: &OperationRequest,
    base: Option<SnapshotRecord>,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    request.validate_profile()?;
    let computed =
        computation::compute_mutation(&crate::compatibility::input(request), base, check)?;
    bind(request, computed, check)
}
pub fn compute_import(
    request: &OperationRequest,
    base: Option<SnapshotRecord>,
    asset: ExportAsset<'_>,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    let computed =
        computation::compute_import(&crate::compatibility::input(request), base, asset, check)?;
    bind(request, computed, check)
}
