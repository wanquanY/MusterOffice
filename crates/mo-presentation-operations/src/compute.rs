use crate::*;
use mo_common::{Digest, DocumentId};
use mo_presentation_edit::{Snapshot, SnapshotRecord, Transaction};
use mo_presentation_model::ValidationLimits;

/// Immutable computation result. It confers no authority to persist or publish.
pub struct MutationCandidate {
    request_digest: Digest,
    base_revision: Option<Digest>,
    base_semantic_digest: Option<Digest>,
    snapshot: SnapshotRecord,
    receipt: MutationReceipt,
}
impl MutationCandidate {
    pub fn request_digest(&self) -> &Digest {
        &self.request_digest
    }
    pub fn document_id(&self) -> &DocumentId {
        &self.snapshot.document.id
    }
    pub fn base_revision(&self) -> Option<&Digest> {
        self.base_revision.as_ref()
    }
    pub fn base_semantic_digest(&self) -> Option<&Digest> {
        self.base_semantic_digest.as_ref()
    }
    pub fn snapshot(&self) -> &SnapshotRecord {
        &self.snapshot
    }
    pub fn into_parts(self) -> (SnapshotRecord, MutationReceipt) {
        (self.snapshot, self.receipt)
    }
    pub fn receipt(&self) -> &MutationReceipt {
        &self.receipt
    }
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), Failure> {
    if check() {
        Err(Failure::new(FailureCode::Cancelled, "operation cancelled"))
    } else {
        Ok(())
    }
}
pub fn compute_mutation(
    request: &Computation<'_>,
    base: Option<SnapshotRecord>,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    cancelled(check)?;
    request.validate_profile()?;
    let request_digest = request
        .digest()
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "canonical operation request"))?;
    let limits = ValidationLimits::default();
    let (snapshot, transaction, base_revision, base_semantic_digest) = match &request.action {
        DocumentAction::Import { .. } => {
            return Err(Failure::new(
                FailureCode::ResourceIncomplete,
                "import requires explicit source bytes",
            ));
        }
        DocumentAction::Create { document } => {
            if document.source_bindings.is_some() {
                return Err(Failure::new(
                    FailureCode::InputInvalid,
                    "source-backed documents require import admission",
                ));
            }
            if base.is_some() {
                return Err(Failure::new(
                    FailureCode::DocumentExists,
                    "document already exists",
                ));
            }
            (
                Snapshot::new(*document.clone(), limits)
                    .map_err(Failure::from)?
                    .into_record(),
                None,
                None,
                None,
            )
        }
        DocumentAction::Apply {
            document_id,
            base_revision,
            operations,
        } => {
            let base =
                base.ok_or_else(|| Failure::new(FailureCode::NotFound, "document is unavailable"))?;
            let base = Snapshot::restore(base, limits).map_err(Failure::from)?;
            cancelled(check)?;
            let transaction = Transaction {
                document_id: document_id.clone(),
                request_id: request.request_id.clone(),
                base_revision: base_revision.clone(),
                operations: operations.clone(),
            };
            let prepared =
                mo_presentation_edit::prepare_cancellable(&base, &transaction, limits, check)
                    .map_err(Failure::from)?;
            (
                prepared.snapshot.into_record(),
                Some(Box::new(prepared.receipt)),
                Some(base_revision.clone()),
                Some(base.semantic_digest().clone()),
            )
        }
        DocumentAction::Export { .. } => {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "export requires delivery computation",
            ));
        }
    };
    cancelled(check)?;
    candidate(
        request_digest,
        snapshot,
        transaction,
        base_revision,
        base_semantic_digest,
        check,
    )
}

fn candidate(
    request_digest: Digest,
    snapshot: SnapshotRecord,
    transaction: Option<Box<mo_presentation_edit::TransactionReceipt>>,
    base_revision: Option<Digest>,
    base_semantic_digest: Option<Digest>,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    let receipt = MutationReceipt {
        document_id: snapshot.document.id.clone(),
        revision: snapshot.revision.clone(),
        semantic_digest: snapshot.semantic_digest.clone(),
        transaction,
    };
    crate::budget::check_size(&snapshot, MAX_OPERATION_BYTES, "snapshot bytes", check)?;
    cancelled(check)?;
    Ok(MutationCandidate {
        request_digest,
        base_revision,
        base_semantic_digest,
        snapshot,
        receipt,
    })
}

/// Import validates caller-provided immutable bytes and returns a document result.
pub fn compute_import(
    request: &Computation<'_>,
    base: Option<SnapshotRecord>,
    asset: ExportAsset<'_>,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    cancelled(check)?;
    request.validate_profile()?;
    let DocumentAction::Import {
        document_id,
        source,
    } = &request.action
    else {
        return Err(Failure::new(
            FailureCode::InputInvalid,
            "source import action required",
        ));
    };
    if base.is_some() {
        return Err(Failure::new(
            FailureCode::DocumentExists,
            "document already exists",
        ));
    }
    if asset.info.id != source.asset_id {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "source asset identity differs",
        ));
    }
    let package = mo_opc::Package::open(
        asset.reader,
        asset.info.descriptor.byte_length.get(),
        Default::default(),
        check,
    )
    .map_err(|e| delivery_failure(mo_presentation_delivery::DeliveryError::Pptx(e.into())))?;
    if package.sha256() != &asset.info.descriptor.sha256 {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "source asset digest differs",
        ));
    }
    let document = mo_pptx::source::document::import_document(
        &package,
        document_id.clone(),
        source.resource_id.clone(),
        Default::default(),
        check,
    )
    .map_err(|e| delivery_failure(mo_presentation_delivery::DeliveryError::Pptx(e)))?;
    if document.resources[&source.resource_id].media_type != asset.info.descriptor.media_type {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "source media type differs",
        ));
    }
    let snapshot = Snapshot::new(document, ValidationLimits::default())
        .map_err(Failure::from)?
        .into_record();
    let digest = request
        .digest()
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "canonical operation request"))?;
    candidate(digest, snapshot, None, None, None, check)
}
