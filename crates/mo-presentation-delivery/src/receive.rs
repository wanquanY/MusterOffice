//! Admission of a transported delivery. This proves byte/reference binding,
//! never transport authority, external application quality or host publication.
mod context;
mod evidence;
mod png;
#[cfg(test)]
mod tests;
use crate::*;
use mo_common::{ByteLength, Digest, DocumentId, RequestId};
use mo_opc::Package;
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use mo_presentation_model::ValidationLimits;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};

/// Pins come from the accepted operation, not from this delivery's own claims.
/// The product separately binds them to its invocation, generation and fence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryExpectation {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub semantic_digest: Digest,
    pub settings_digest: Digest,
    pub renderer: RendererIdentity,
}

/// Only immutable, retained, authorized bytes. Each returned reader must keep
/// the same bytes for the entire inspection and subsequent host commit.
pub trait DeliverySource {
    fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError>;
}

/// A diagnostic report, not a transferable grant or a substitute for a commit.
/// Claims remain producer declarations; this checker does not upgrade them.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptInspection {
    pub profile: String,
    pub bundle_digest: Digest,
    pub document_id: DocumentId,
    pub revision: Digest,
    pub semantic_digest: Digest,
    pub settings_digest: Digest,
    pub assets_verified: usize,
    pub total_bytes: ByteLength,
    pub pages: usize,
    pub declared_claims: Vec<Claim>,
}

/// Constructed only by the actual reader. Products must still perform their
/// atomic authorization/cancellation/pin/fence checks before publication.
pub struct ReceivedDelivery {
    report: ReceiptInspection,
    snapshot: SnapshotRecord,
}
impl ReceivedDelivery {
    pub fn report(&self) -> &ReceiptInspection {
        &self.report
    }
    pub fn snapshot(&self) -> &SnapshotRecord {
        &self.snapshot
    }
}

struct Inputs<'a> {
    assets: BTreeMap<&'a RequestId, (&'a DeliveryAsset, Content<'a>)>,
    used: BTreeSet<&'a RequestId>,
    limits: DeliveryLimits,
    check: &'a dyn Fn() -> bool,
}
impl<'a> Inputs<'a> {
    fn use_asset(&mut self, id: &RequestId) -> Result<&'a DeliveryAsset, DeliveryError> {
        let (asset, _) = self
            .assets
            .get(id)
            .ok_or(DeliveryError::Invalid("missing delivery asset"))?;
        self.used.insert(&asset.id);
        Ok(*asset)
    }
    fn role(
        &mut self,
        id: &RequestId,
        role: AssetRole,
        mime: &str,
    ) -> Result<&'a DeliveryAsset, DeliveryError> {
        let asset = self.use_asset(id)?;
        if asset.role != role || asset.media_type != mime {
            return Err(DeliveryError::Invalid("delivery asset role or media type"));
        }
        Ok(asset)
    }
    fn unique(&mut self, role: AssetRole, mime: &str) -> Result<&'a DeliveryAsset, DeliveryError> {
        let mut matches = self
            .assets
            .values()
            .filter(|(a, _)| a.role == role && a.media_type == mime);
        let asset = matches
            .next()
            .ok_or(DeliveryError::Invalid("required delivery asset"))?
            .0;
        if matches.next().is_some() {
            return Err(DeliveryError::Invalid("ambiguous delivery asset"));
        }
        self.used.insert(&asset.id);
        Ok(asset)
    }
    fn json<T: DeserializeOwned>(&self, id: &RequestId) -> Result<T, DeliveryError> {
        let (_, content) = self
            .assets
            .get(id)
            .ok_or(DeliveryError::Invalid("missing delivery JSON"))?;
        if content.byte_length > self.limits.max_model_bytes {
            return Err(DeliveryError::Limit("delivery JSON bytes"));
        }
        let len = usize::try_from(content.byte_length)
            .map_err(|_| DeliveryError::Limit("delivery JSON bytes"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(len)
            .map_err(|_| DeliveryError::Limit("delivery JSON allocation"))?;
        bytes.resize(len, 0);
        for (i, block) in bytes.chunks_mut(65536).enumerate() {
            cancel(self.check)?;
            content.reader.read_exact_at(block, (i * 65536) as u64)?;
        }
        cancel(self.check)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| DeliveryError::Invalid("delivery JSON UTF-8"))?;
        mo_common::from_json_str(text).map_err(|_| DeliveryError::Invalid("delivery JSON schema"))
    }
}

pub fn inspect(
    bundle: &DeliveryBundle,
    expected: &DeliveryExpectation,
    source: &dyn DeliverySource,
    limits: DeliveryLimits,
    check: &dyn Fn() -> bool,
) -> Result<ReceivedDelivery, DeliveryError> {
    cancel(check)?;
    if bundle.version != "musteroffice.bundle/1-draft"
        || bundle.profile_id != PROFILE
        || bundle.document.document_id != expected.document_id
        || bundle.document.revision != expected.revision
        || bundle.versions.document_schema != "musteroffice.presentation/0.1-draft"
        || bundle.versions.operation_schema != "musteroffice.operations/1-draft"
        || bundle.versions.rules != PROFILE
        || bundle.versions.engine != concat!("MusterOffice/", env!("CARGO_PKG_VERSION"))
    {
        return Err(DeliveryError::Invalid(
            "delivery version or accepted operation pin",
        ));
    }
    if bundle.assets.is_empty()
        || bundle.assets.len() > limits.max_artifacts
        || bundle.previews.is_empty()
        || bundle.previews.len() > limits.max_pages
    {
        return Err(DeliveryError::Limit("delivery count"));
    }
    // Preflight every declaration before opening any resource.
    let mut ids = BTreeSet::new();
    let mut total = 0u64;
    for asset in &bundle.assets {
        cancel(check)?;
        if !ids.insert(&asset.id) {
            return Err(DeliveryError::Invalid("duplicate delivery asset"));
        }
        if asset.byte_length.get() > limits.max_asset_bytes {
            return Err(DeliveryError::Limit("delivery asset bytes"));
        }
        total = total
            .checked_add(asset.byte_length.get())
            .ok_or(DeliveryError::Limit("delivery total bytes"))?;
        if total > limits.max_total_bytes {
            return Err(DeliveryError::Limit("delivery total bytes"));
        }
    }
    let mut input = Inputs {
        assets: BTreeMap::new(),
        used: BTreeSet::new(),
        limits,
        check,
    };
    for asset in &bundle.assets {
        cancel(check)?;
        let content = source.open(&asset.id)?;
        if content.byte_length != asset.byte_length.get()
            || artifact::digest(content.reader, content.byte_length, check)? != asset.sha256
        {
            return Err(DeliveryError::Invalid("actual delivery bytes differ"));
        }
        input.assets.insert(&asset.id, (asset, content));
    }
    input.role(
        &bundle.document.model_asset_id,
        AssetRole::EditableDocument,
        MODEL_MIME,
    )?;
    let snapshot: SnapshotRecord = input.json(&bundle.document.model_asset_id)?;
    let snapshot = Snapshot::restore(snapshot, ValidationLimits::default())
        .map_err(|_| DeliveryError::Invalid("delivery model semantic digest"))?
        .into_record();
    if snapshot.semantic_digest != expected.semantic_digest {
        return Err(DeliveryError::Invalid("accepted model semantic digest"));
    }
    crate::integrity::validate(bundle, &snapshot)?;
    cancel(check)?;
    let context = context::validate(&mut input, bundle, &snapshot, expected)?;
    input.role(&bundle.pptx_asset_id, AssetRole::Pptx, PPTX_MIME)?;
    let (_, pptx) = &input.assets[&bundle.pptx_asset_id];
    let package = Package::open(
        pptx.reader,
        pptx.byte_length,
        mo_opc::PackageLimits {
            max_package_bytes: limits.max_asset_bytes,
            ..Default::default()
        },
        check,
    )
    .map_err(mo_pptx::PptxError::from)?;
    let index = mo_pptx::source::inspect_source(&package, Default::default(), check)?;
    if index.slides.len() != snapshot.document.slide_order.len()
        || index.page_size != Some(snapshot.document.page_size)
    {
        return Err(DeliveryError::Invalid("PPTX page coverage"));
    }
    evidence::validate(&mut input, bundle, &snapshot, &context, &index)?;
    if input.used.len() != input.assets.len() {
        return Err(DeliveryError::Invalid("unreferenced delivery asset"));
    }
    cancel(check)?;
    let bundle_digest = mo_common::digest("musteroffice.delivery-bundle/1", bundle)
        .map_err(|_| DeliveryError::Serialization)?;
    Ok(ReceivedDelivery {
        report: ReceiptInspection {
            profile: "delivery-bytes-reference-binding-v1-draft".into(),
            bundle_digest,
            document_id: snapshot.document.id.clone(),
            revision: snapshot.revision.clone(),
            semantic_digest: snapshot.semantic_digest.clone(),
            settings_digest: expected.settings_digest.clone(),
            assets_verified: bundle.assets.len(),
            total_bytes: ByteLength::new(total),
            pages: bundle.previews.len(),
            declared_claims: bundle.claims.clone(),
        },
        snapshot,
    })
}
