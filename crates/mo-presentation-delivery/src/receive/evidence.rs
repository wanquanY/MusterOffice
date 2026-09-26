use super::*;
use mo_common::SlideId;
use mo_pptx::source::SourceIndex;
use mo_presentation_compile::source_resource_page::SourceResourcePageRasterInfo;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Structure {
    profile: String,
    actual_stored_bytes_verified: bool,
    full_presentation_xsd: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Quality {
    format: String,
    subject_sha256: Digest,
    model_semantic_digest: Digest,
    context_asset_id: RequestId,
    structure: Structure,
    page_coverage: Vec<SlideId>,
    previews: Vec<RequestId>,
    layout_quality_proven: bool,
    native_editability_proven: bool,
    playback_proven: bool,
    target_application_proven: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewEvidence {
    format: String,
    page_id: SlideId,
    pptx_sha256: Digest,
    #[serde(default)]
    author_plan_sha256: Option<Digest>,
    #[serde(default)]
    plan_sha256: Option<Digest>,
    preview_asset: DeliveryAsset,
    render: SourceResourcePageRasterInfo,
}
pub(super) fn validate(
    input: &mut Inputs<'_>,
    bundle: &DeliveryBundle,
    snapshot: &SnapshotRecord,
    context: &context::BoundContext,
    index: &SourceIndex,
) -> Result<(), DeliveryError> {
    let asset = input.unique(
        AssetRole::QualityReport,
        "application/vnd.musteroffice.quality+json",
    )?;
    let quality: Quality = input.json(&asset.id)?;
    let pptx_sha = &input.assets[&bundle.pptx_asset_id].0.sha256;
    if quality.format != "musteroffice.delivery-evidence/1-draft"
        || quality.subject_sha256 != *pptx_sha
        || quality.model_semantic_digest != snapshot.semantic_digest
        || quality.context_asset_id != context.id
        || quality.page_coverage != snapshot.document.slide_order
        || quality.previews.len() != bundle.previews.len()
        || quality.structure.profile != "opc-zip-xml-graph-digest-v1-draft"
        || !quality.structure.actual_stored_bytes_verified
        || quality.structure.full_presentation_xsd
        || quality.layout_quality_proven
        || quality.native_editability_proven
        || quality.playback_proven
        || quality.target_application_proven
    {
        return Err(DeliveryError::Invalid(
            "delivery evidence coverage or unsupported assertion",
        ));
    }
    // The current profile cannot produce external application/playback proof.
    // A future evidence profile must define and validate those records itself.
    for claim in &bundle.claims {
        if claim.evidence_asset_ids != [asset.id.clone()]
            || (claim.kind == ClaimKind::Structure
                && (claim.status != ClaimStatus::Passed
                    || claim.basis != ClaimBasis::StaticInspection
                    || claim.profile_id != quality.structure.profile))
            || (claim.kind != ClaimKind::Structure
                && (claim.status != ClaimStatus::NotProven
                    || claim.basis != ClaimBasis::None
                    || claim.profile_id != PROFILE))
        {
            return Err(DeliveryError::Invalid(
                "claim does not match supported evidence profile",
            ));
        }
    }
    let viewport = preview::viewport(
        snapshot.document.page_size,
        context.value.settings.preview_width,
    )?;
    let plan_identity = {
        let resources = ReceivedResources {
            input,
            context: &context.value,
        };
        mo_pptx::PresentationPlan::new(
            &snapshot.document,
            &context.value.settings.defaults,
            &resources,
            Default::default(),
            Default::default(),
            input.check,
        )?
        .identity()
        .clone()
    };
    let mut evidence_ids = BTreeSet::new();
    for ((preview, evidence_id), slide) in bundle
        .previews
        .iter()
        .zip(&quality.previews)
        .zip(&index.slides)
    {
        cancel(input.check)?;
        if !evidence_ids.insert(evidence_id) {
            return Err(DeliveryError::Invalid("duplicate preview evidence"));
        }
        input.role(evidence_id, AssetRole::QualityReport, "application/json")?;
        let evidence: PreviewEvidence = input.json(evidence_id)?;
        let image = input.role(&preview.image_asset_id, AssetRole::Preview, "image/png")?;
        let page = &evidence.render.page.page;
        let raster = &evidence.render.page.scene.raster;
        // Legacy evidence binds source rendering to file bytes; the direct
        // profile binds semantic rendering to the independently rebuilt plan.
        let identity_matches = match evidence.format.as_str() {
            "musteroffice.preview-evidence/1-draft" => {
                evidence.plan_sha256.is_none()
                    && evidence.author_plan_sha256.is_none()
                    && page.source_sha256 == *pptx_sha
            }
            "musteroffice.preview-evidence/2-draft" => {
                snapshot.document.source_bindings.is_none()
                    && evidence.plan_sha256.is_none()
                    && evidence.author_plan_sha256.as_ref() == Some(&plan_identity)
                    && page.source_sha256 == plan_identity
            }
            "musteroffice.preview-evidence/3-draft" => {
                evidence.author_plan_sha256.is_none()
                    && evidence.plan_sha256.as_ref() == Some(&plan_identity)
                    && page.source_sha256 == plan_identity
            }
            _ => false,
        };
        if !identity_matches
            || evidence.page_id != preview.page_id
            || evidence.pptx_sha256 != *pptx_sha
            || serde_json::to_value(&evidence.preview_asset)
                .map_err(|_| DeliveryError::Serialization)?
                != serde_json::to_value(image).map_err(|_| DeliveryError::Serialization)?
            || page.slide != slide.part
            || page.hidden_slide != snapshot.document.slides[&preview.page_id].hidden
            || evidence.render.profile != mo_presentation_compile::source_resource_page::PROFILE
            || evidence.render.page.scene.profile != mo_render::PROFILE
            || !mo_raster::accepts_profile(&raster.profile, true)
            || preview.width != viewport.width
            || preview.height != viewport.height
            || raster.width != preview.width
            || raster.height != preview.height
            || raster.byte_length.get() != u64::from(preview.width) * u64::from(preview.height) * 4
        {
            return Err(DeliveryError::Invalid("preview evidence binding"));
        }
        let (_, content) = &input.assets[&preview.image_asset_id];
        super::png::verify(
            content,
            preview.width,
            preview.height,
            &raster.sha256,
            input.check,
        )?;
    }
    Ok(())
}

struct ReceivedResources<'a, 'b> {
    input: &'a Inputs<'b>,
    context: &'a context::Context,
}
impl mo_pptx::Resources for ReceivedResources<'_, '_> {
    fn open(
        &self,
        id: &mo_common::ResourceId,
    ) -> Result<mo_pptx::ResourceData<'_>, mo_pptx::PptxError> {
        let asset = self
            .context
            .resource_assets
            .get(id)
            .and_then(|id| self.input.assets.get(id))
            .ok_or_else(|| mo_pptx::PptxError::ResourceRequired(id.clone()))?;
        Ok(mo_pptx::ResourceData {
            reader: asset.1.reader,
            byte_length: asset.1.byte_length,
        })
    }
}
