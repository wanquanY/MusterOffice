use crate::{NativeExportCandidate, NativeExporter};
use mo_common::{DocumentId, RequestId, ResourceId};
use mo_presentation_delivery::DeliverySettings;
use mo_presentation_edit::{OperationEntry, Snapshot, SnapshotRecord};
use mo_presentation_model::{Document, ValidationLimits};
use mo_presentation_operations::{
    AssetBinding, AssetId, Computation, ContractVersion, DocumentAction, ExportAsset, ExportAssets,
    ExportSettings, Failure, FailureCode, MutationReceipt, OperationProfile, OperationRequest,
    compute_import, compute_mutation,
};

/// A caller-owned, in-memory presentation. Dropping it releases the document;
/// no database, task, account, filesystem or product publication is created.
pub struct Presentation {
    snapshot: SnapshotRecord,
}

/// Export content choices. The configured executor supplies its own renderer
/// identity; neither paths nor executable choices come from these options.
pub struct ExportOptions {
    pub delivery: DeliverySettings,
    pub resources: Vec<AssetBinding>,
    pub font_asset_id: Option<AssetId>,
}

impl Presentation {
    pub fn create(document: Document, cancelled: &dyn Fn() -> bool) -> Result<Self, Failure> {
        let action = DocumentAction::Create {
            document: Box::new(document),
        };
        let request_id = RequestId::new("sdk:create").expect("static request identifier");
        let result = compute_mutation(
            &Computation {
                request_id: &request_id,
                profile_id: OperationProfile::AuthorModel,
                action: &action,
            },
            None,
            cancelled,
        )?;
        Ok(Self {
            snapshot: result.into_parts().0,
        })
    }

    pub fn import(
        document_id: DocumentId,
        resource_id: ResourceId,
        source: ExportAsset<'_>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, Failure> {
        let action = DocumentAction::Import {
            document_id,
            source: AssetBinding {
                resource_id,
                asset_id: source.info.id.clone(),
            },
        };
        let request_id = RequestId::new("sdk:import").expect("static request identifier");
        let result = compute_import(
            &Computation {
                request_id: &request_id,
                profile_id: OperationProfile::AuthorModel,
                action: &action,
            },
            None,
            source,
            cancelled,
        )?;
        Ok(Self {
            snapshot: result.into_parts().0,
        })
    }

    /// The product chooses which revision to supply. This checks model and
    /// semantic integrity; it does not authenticate history or a durable head.
    pub fn from_snapshot(snapshot: SnapshotRecord) -> Result<Self, Failure> {
        let snapshot = Snapshot::restore(snapshot, ValidationLimits::default())?.into_record();
        Ok(Self { snapshot })
    }

    pub fn snapshot(&self) -> &SnapshotRecord {
        &self.snapshot
    }
    pub fn document(&self) -> &Document {
        &self.snapshot.document
    }
    pub fn into_snapshot(self) -> SnapshotRecord {
        self.snapshot
    }

    /// An all-or-nothing edit of this value. The caller owns retries and any
    /// persistent CAS; a request ID is only a document transaction identity.
    pub fn edit(
        &mut self,
        request_id: RequestId,
        operations: Vec<OperationEntry>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<MutationReceipt, Failure> {
        let action = DocumentAction::Apply {
            document_id: self.snapshot.document.id.clone(),
            base_revision: self.snapshot.revision.clone(),
            operations,
        };
        let result = compute_mutation(
            &Computation {
                request_id: &request_id,
                profile_id: OperationProfile::AuthorModel,
                action: &action,
            },
            Some(self.snapshot.clone()),
            cancelled,
        )?;
        // Last cancellation point before replacing the caller's in-memory value.
        if cancelled() {
            return Err(Failure::new(FailureCode::Cancelled, "edit cancelled"));
        }
        let (snapshot, receipt) = result.into_parts();
        self.snapshot = snapshot;
        Ok(receipt)
    }

    pub fn export(
        &self,
        exporter: &NativeExporter,
        request_id: RequestId,
        options: ExportOptions,
        assets: &dyn ExportAssets,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<NativeExportCandidate, Failure> {
        let request = OperationRequest {
            contract_version: ContractVersion::V1,
            request_id,
            profile_id: OperationProfile::ResourceDelivery,
            action: DocumentAction::Export {
                document_id: self.snapshot.document.id.clone(),
                base_revision: self.snapshot.revision.clone(),
                settings: Box::new(ExportSettings {
                    delivery: options.delivery,
                    resources: options.resources,
                    font_asset_id: options.font_asset_id,
                    renderer: exporter.renderer_identity(),
                }),
            },
        };
        exporter.prepare(&request, self.snapshot.clone(), assets, cancelled)
    }
}
