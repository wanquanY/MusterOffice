//! Pure export request binding. Asset authority and publication stay in hosts.
use crate::*;
use mo_common::{Digest, DocumentId, ResourceId};
use mo_opc::{ReaderAt, ResultSink};
use mo_pptx::{PptxError, ResourceData, Resources};
use mo_presentation_delivery::{
    Content, DeliveryBundle, DeliveryCandidate, DeliveryError, DeliveryInputs, DeliveryLimits,
    DeliverySettings, OutputStore, PreviewRenderer, RendererIdentity,
};
use mo_presentation_edit::SnapshotRecord;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportSettings {
    pub delivery: DeliverySettings,
    pub resources: Vec<AssetBinding>,
    pub font_asset_id: Option<AssetId>,
    /// Expected identity, never an executable path or permission grant.
    pub renderer: RendererIdentity,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportReceipt {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub semantic_digest: Digest,
    pub bundle: DeliveryBundle,
}

pub struct ExportAsset<'a> {
    pub info: &'a AssetInfo,
    pub reader: &'a dyn ReaderAt,
}
/// The caller resolves immutable bytes; this interface grants no access rights.
pub trait ExportAssets<E = Failure> {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, E>;
}
struct Inputs<'a>(BTreeMap<ResourceId, ExportAsset<'a>>);
impl Resources for Inputs<'_> {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        let asset = self
            .0
            .get(id)
            .ok_or_else(|| PptxError::ResourceRequired(id.clone()))?;
        Ok(ResourceData {
            reader: asset.reader,
            byte_length: asset.info.descriptor.byte_length.get(),
        })
    }
}
pub struct ExportCandidate<R> {
    request_digest: Digest,
    delivery: DeliveryCandidate<R>,
}
impl<R> ExportCandidate<R> {
    pub fn request_digest(&self) -> &Digest {
        &self.request_digest
    }
    pub fn delivery(&self) -> &DeliveryCandidate<R> {
        &self.delivery
    }
    pub fn receipt(&self) -> ExportReceipt {
        ExportReceipt {
            document_id: self.delivery.bundle().document.document_id.clone(),
            revision: self.delivery.bundle().document.revision.clone(),
            semantic_digest: self.delivery.semantic_digest().clone(),
            bundle: self.delivery.bundle().clone(),
        }
    }
}

fn invalid(message: &'static str) -> Failure {
    Failure::new(FailureCode::InputInvalid, message)
}
fn asset<'a, E: From<Failure>>(
    assets: &'a dyn ExportAssets<E>,
    id: &AssetId,
) -> Result<ExportAsset<'a>, E> {
    let value = assets.get(id)?;
    if &value.info.id != id {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "resolved asset identity differs",
        )
        .into());
    }
    Ok(value)
}

pub fn compute_export<S: OutputStore, E: From<Failure> + From<DeliveryError>>(
    request: &Computation<'_>,
    snapshot: SnapshotRecord,
    assets: &dyn ExportAssets<E>,
    store: &mut S,
    renderer: &mut dyn PreviewRenderer,
    limits: DeliveryLimits,
    check: &dyn Fn() -> bool,
) -> Result<ExportCandidate<<S::Sink as ResultSink>::Reader>, E> {
    request.validate_profile()?;
    if check() {
        return Err(Failure::new(FailureCode::Cancelled, "export cancelled").into());
    }
    let DocumentAction::Export {
        document_id,
        base_revision,
        settings,
    } = &request.action
    else {
        return Err(invalid("export action required").into());
    };
    if &snapshot.document.id != document_id || &snapshot.revision != base_revision {
        return Err(Failure::new(
            FailureCode::RevisionConflict,
            "export snapshot differs from requested revision",
        )
        .into());
    }
    if renderer.identity() != settings.renderer {
        return Err(Failure::new(
            FailureCode::ExecutorMismatch,
            "export renderer differs from accepted request",
        )
        .into());
    }
    if settings.resources.len() != snapshot.document.resources.len() {
        return Err(Failure::new(
            FailureCode::ResourceIncomplete,
            "export resource closure differs",
        )
        .into());
    }
    let mut resources = BTreeMap::new();
    for binding in &settings.resources {
        let definition = snapshot
            .document
            .resources
            .get(&binding.resource_id)
            .ok_or_else(|| invalid("resource binding outside document"))?;
        let value = asset(assets, &binding.asset_id)?;
        if definition.sha256 != value.info.descriptor.sha256
            || definition.media_type != value.info.descriptor.media_type
            || resources
                .insert(binding.resource_id.clone(), value)
                .is_some()
        {
            return Err(Failure::new(
                FailureCode::ResourceConflict,
                "export resource identity differs",
            )
            .into());
        }
    }
    let fonts = settings
        .font_asset_id
        .as_ref()
        .map(|id| asset(assets, id))
        .transpose()?;
    let empty: Vec<u8> = Vec::new();
    let (reader, length, font_digest): (&dyn ReaderAt, _, _) = match &fonts {
        Some(f) => (
            f.reader,
            f.info.descriptor.byte_length.get(),
            f.info.descriptor.sha256.clone(),
        ),
        None => (&empty, 0, Digest::from_sha256(Sha256::digest([]).into())),
    };
    let expected_settings = settings
        .delivery
        .input_digest(&font_digest, &settings.renderer)
        .map_err(|_| invalid("canonical export settings"))?;
    let request_digest = request
        .digest()
        .map_err(|_| invalid("canonical export request"))?;
    let delivery = mo_presentation_delivery::build(
        DeliveryInputs {
            snapshot,
            settings: &settings.delivery,
            resources: &Inputs(resources),
            fonts: Content {
                reader,
                byte_length: length,
            },
        },
        store,
        renderer,
        limits,
        check,
    )
    .map_err(E::from)?;
    if delivery.settings_digest() != &expected_settings {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "export font bytes differ from resolved asset",
        )
        .into());
    }
    Ok(ExportCandidate {
        request_digest,
        delivery,
    })
}
