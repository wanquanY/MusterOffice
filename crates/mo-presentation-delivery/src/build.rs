use crate::{
    artifact::{self, Spec},
    preview, *,
};
use mo_common::Digest;
use mo_opc::ResultSink;
use mo_pptx::{PptxLimits, Resources, source::inspect_source};
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use mo_presentation_model::{ResourceKind, ValidationLimits};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

pub struct DeliveryInputs<'a> {
    pub snapshot: SnapshotRecord,
    pub settings: &'a DeliverySettings,
    pub resources: &'a dyn Resources,
    pub fonts: Content<'a>,
}
/// The complete immutable calculation result. Hosts still bind accepted
/// request/executor/fence/cancellation and atomically commit every asset.
pub struct DeliveryCandidate<R> {
    bundle: DeliveryBundle,
    artifacts: Vec<ProducedArtifact<R>>,
    semantic_digest: Digest,
    settings_digest: Digest,
}
impl<R> DeliveryCandidate<R> {
    pub fn bundle(&self) -> &DeliveryBundle {
        &self.bundle
    }
    pub fn artifacts(&self) -> &[ProducedArtifact<R>] {
        &self.artifacts
    }
    pub fn semantic_digest(&self) -> &Digest {
        &self.semantic_digest
    }
    pub fn settings_digest(&self) -> &Digest {
        &self.settings_digest
    }
    pub fn into_parts(self) -> (DeliveryBundle, Vec<ProducedArtifact<R>>) {
        (self.bundle, self.artifacts)
    }
}
struct Collector<'a, S: OutputStore> {
    store: &'a mut S,
    limits: DeliveryLimits,
    total: u64,
    outputs: Vec<ProducedArtifact<<S::Sink as ResultSink>::Reader>>,
    check: &'a dyn Fn() -> bool,
}
impl<S: OutputStore> Collector<'_, S> {
    fn budget(&self) -> Result<u64, DeliveryError> {
        cancel(self.check)?;
        if self.outputs.len() >= self.limits.max_artifacts {
            return Err(DeliveryError::Limit("artifact count"));
        }
        Ok(self.limits.max_asset_bytes.min(
            self.limits
                .max_total_bytes
                .checked_sub(self.total)
                .ok_or(DeliveryError::Limit("total bytes"))?,
        ))
    }
    fn add(
        &mut self,
        output: ProducedArtifact<<S::Sink as ResultSink>::Reader>,
    ) -> Result<DeliveryAsset, DeliveryError> {
        if output.asset.byte_length.get() > self.budget()? {
            return Err(DeliveryError::Limit("total bytes"));
        }
        if self
            .outputs
            .iter()
            .any(|a| a.name == output.name || a.asset.id == output.asset.id)
        {
            return Err(DeliveryError::Invalid("duplicate output identity"));
        }
        self.total += output.asset.byte_length.get();
        let asset = output.asset.clone();
        self.outputs.push(output);
        Ok(asset)
    }
    fn copy(
        &mut self,
        spec: Spec<'_>,
        content: Content<'_>,
        expected: Option<&Digest>,
    ) -> Result<DeliveryAsset, DeliveryError> {
        let budget = self.budget()?;
        let output = artifact::copy(self.store, spec, content, expected, budget, self.check)?;
        self.add(output)
    }
    fn json(
        &mut self,
        name: &str,
        mime: &str,
        role: AssetRole,
        value: &impl Serialize,
    ) -> Result<DeliveryAsset, DeliveryError> {
        let limit = self.budget()?.min(self.limits.max_model_bytes);
        let bytes = json_bytes(value, limit)?;
        self.copy(
            Spec { name, mime, role },
            Content {
                reader: &bytes,
                byte_length: bytes.len() as u64,
            },
            None,
        )
    }
}
struct JsonBytes {
    bytes: Vec<u8>,
    limit: u64,
    exceeded: bool,
}
impl Write for JsonBytes {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.bytes.len() as u64 + input.len() as u64 > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("JSON byte limit"));
        }
        self.bytes
            .try_reserve(input.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn json_bytes(value: &impl Serialize, limit: u64) -> Result<Vec<u8>, DeliveryError> {
    let mut out = JsonBytes {
        bytes: vec![],
        limit,
        exceeded: false,
    };
    if serde_json::to_writer(&mut out, value).is_err() {
        return Err(if out.exceeded {
            DeliveryError::Limit("JSON bytes")
        } else {
            DeliveryError::Serialization
        });
    }
    Ok(out.bytes)
}

pub fn build<S: OutputStore>(
    inputs: DeliveryInputs<'_>,
    store: &mut S,
    renderer: &mut dyn PreviewRenderer,
    limits: DeliveryLimits,
    check: &dyn Fn() -> bool,
) -> Result<DeliveryCandidate<<S::Sink as ResultSink>::Reader>, DeliveryError> {
    cancel(check)?;
    let snapshot = Snapshot::restore(inputs.snapshot, ValidationLimits::default())
        .map_err(|_| DeliveryError::Invalid("immutable document snapshot"))?
        .into_record();
    let document = &snapshot.document;
    if document.slide_order.is_empty() || document.slide_order.len() > limits.max_pages {
        return Err(DeliveryError::Limit("delivery page count"));
    }
    let count = (6usize + usize::from(inputs.fonts.byte_length > 0))
        .checked_add(document.resources.len())
        .and_then(|n| n.checked_add(document.slide_order.len().checked_mul(2)?))
        .ok_or(DeliveryError::Limit("artifact count"))?;
    if count > limits.max_artifacts {
        return Err(DeliveryError::Limit("artifact count"));
    }
    if inputs.fonts.byte_length > limits.max_font_bytes
        || (inputs.settings.fonts.is_none() && inputs.fonts.byte_length != 0)
    {
        return Err(DeliveryError::Invalid("explicit font input"));
    }
    let viewport = preview::viewport(document.page_size, inputs.settings.preview_width)?;
    let font_digest = artifact::digest(inputs.fonts.reader, inputs.fonts.byte_length, check)?;
    let renderer_identity = renderer.identity();
    if renderer_identity.profile != mo_presentation_compile::source_resource_page::PROFILE {
        return Err(DeliveryError::Invalid("preview renderer profile"));
    }
    let settings_digest = inputs
        .settings
        .input_digest(&font_digest, &renderer_identity)
        .map_err(|_| DeliveryError::Serialization)?;
    let mut outputs = Collector {
        store,
        limits,
        total: 0,
        outputs: vec![],
        check,
    };
    let mut pptx_limits = PptxLimits::default();
    let budget = outputs.budget()?;
    pptx_limits.package.max_package_bytes = pptx_limits.package.max_package_bytes.min(budget);
    let sink = outputs.store.create(
        "presentation",
        PPTX_MIME,
        pptx_limits.package.max_package_bytes,
    )?;
    let plan = mo_pptx::AuthorPlan::new(
        document,
        &inputs.settings.defaults,
        pptx_limits.document,
        check,
    )?;
    let package =
        mo_pptx::export_plan_to(&plan, inputs.resources, sink, pptx_limits.package, check)?;
    let index = inspect_source(package.package(), Default::default(), check)?;
    if index.slides.len() != document.slide_order.len()
        || index.page_size != Some(document.page_size)
    {
        return Err(DeliveryError::Invalid("exported page coverage"));
    }
    let pptx_asset = artifact::metadata(
        "presentation",
        PPTX_MIME,
        AssetRole::Pptx,
        package.receipt().sha256.clone(),
        package.receipt().byte_length,
    )?;
    // Count the verified file independently of the direct semantic render input.
    if pptx_asset.byte_length.get() > outputs.budget()? {
        return Err(DeliveryError::Limit("PPTX bytes"));
    }
    outputs.total += pptx_asset.byte_length.get();

    let model = outputs.json("model", MODEL_MIME, AssetRole::EditableDocument, &snapshot)?;
    let mut resources = BTreeMap::new();
    for (ordinal, (id, resource)) in document.resources.iter().enumerate() {
        cancel(check)?;
        let data = inputs.resources.open(id)?;
        let role = match resource.kind {
            ResourceKind::Picture => AssetRole::Image,
            ResourceKind::Font => AssetRole::Font,
            ResourceKind::Audio => AssetRole::Audio,
            ResourceKind::Video => AssetRole::Video,
            ResourceKind::SourcePackage => AssetRole::Source,
            ResourceKind::EmbeddedWorkbook => AssetRole::Embedded,
            ResourceKind::Model3d => AssetRole::Model3d,
        };
        let asset = outputs.copy(
            Spec {
                name: &format!("resource:{ordinal}"),
                mime: &resource.media_type,
                role,
            },
            Content {
                reader: data.reader,
                byte_length: data.byte_length,
            },
            Some(&resource.sha256),
        )?;
        resources.insert(id.clone(), asset.id);
    }
    let font_bundle = if inputs.fonts.byte_length > 0 {
        Some(
            outputs
                .copy(
                    Spec {
                        name: "font-bundle",
                        mime: "application/octet-stream",
                        role: AssetRole::Other,
                    },
                    Content {
                        reader: inputs.fonts.reader,
                        byte_length: inputs.fonts.byte_length,
                    },
                    Some(&font_digest),
                )?
                .id,
        )
    } else {
        None
    };
    let font_profile=outputs.json("font-profile","application/vnd.musteroffice.font-profile+json",AssetRole::Other,
        &serde_json::json!({"format":"musteroffice.font-profile/1-draft","manifest":inputs.settings.fonts,"bundleAssetId":font_bundle,"bundleSha256":font_digest,"byteLength":inputs.fonts.byte_length.to_string()}))?;
    let registry = outputs.json(
        "feature-registry",
        crate::registry::MIME,
        AssetRole::Other,
        &crate::registry::snapshot(),
    )?;
    let context=outputs.json("delivery-context",CONTEXT_MIME,AssetRole::Other,
        &serde_json::json!({"format":"musteroffice.presentation-context/1-draft","settings":inputs.settings,"settingsDigest":settings_digest,"previewRenderer":renderer_identity,"modelAssetId":model.id,"resourceAssets":resources,"fontProfileAssetId":font_profile.id,"fontBundleAssetId":font_bundle,"registryAssetId":registry.id}))?;

    let mut previews = Vec::new();
    let mut preview_evidence = Vec::new();
    let requests: Vec<_> = plan
        .declarations()
        .slides
        .iter()
        .map(|slide| preview::request(inputs.settings, &viewport, plan.identity(), &slide.part))
        .collect();
    if renderer.identity() != renderer_identity {
        return Err(DeliveryError::Invalid("preview renderer identity changed"));
    }
    renderer.render_pages(
        &requests,
        PreviewInput::Author { plan: &plan, resources: inputs.resources },
        // ReaderAt is an immutable resource. The copied font artifact was
        // verified against font_digest; face identities are checked at prepare.
        PreviewFonts { manifest: inputs.settings.fonts.as_ref(), content: Content { reader: inputs.fonts.reader, byte_length: inputs.fonts.byte_length } },
        check,
        &mut |ordinal, image| {
        cancel(check)?;
        if ordinal != previews.len() { return Err(DeliveryError::Invalid("preview page sequence")); }
        let request = requests.get(ordinal).ok_or(DeliveryError::Invalid("unexpected preview page"))?;
        let page_id = &document.slide_order[ordinal];
        preview::validate(request, &image, check)?;
        if image.info.page.page.hidden_slide != document.slides[page_id].hidden {
            return Err(DeliveryError::Invalid("preview visibility binding"));
        }
        let name = format!("preview:{ordinal}");
        let budget = outputs.budget()?;
        let mut sink = outputs.store.create(&name, "image/png", budget)?;
        let written = mo_image::png::encode_to(
            &image.pixels,
            viewport.width,
            viewport.height,
            &mut sink,
            budget,
            check,
        )?;
        let asset = artifact::seal(
            sink,
            Spec {
                name: &name,
                mime: "image/png",
                role: AssetRole::Preview,
            },
            &written.sha256,
            written.byte_length,
            check,
        )?;
        let asset = outputs.add(asset)?;
        let evidence=outputs.json(&format!("preview-evidence:{ordinal}"),"application/json",AssetRole::QualityReport,
            &serde_json::json!({"format":"musteroffice.preview-evidence/2-draft","authorPlanSha256":plan.identity(),"pageId":page_id,"pptxSha256":pptx_asset.sha256,"previewAsset":asset,"render":image.info}))?;
        preview_evidence.push(evidence.id);
        previews.push(Preview {
            page_id: page_id.clone(),
            image_asset_id: asset.id,
            width: viewport.width,
            height: viewport.height,
            sample: PreviewSample::Editor {},
        });
        Ok(())
    })?;
    if previews.len() != requests.len() {
        return Err(DeliveryError::Invalid("missing preview pages"));
    }
    if renderer.identity() != renderer_identity {
        return Err(DeliveryError::Invalid("preview renderer identity changed"));
    }
    let quality=outputs.json("quality","application/vnd.musteroffice.quality+json",AssetRole::QualityReport,
        &serde_json::json!({"format":"musteroffice.delivery-evidence/1-draft","subjectSha256":pptx_asset.sha256,"modelSemanticDigest":snapshot.semantic_digest,"contextAssetId":context.id,"structure":{"profile":"opc-zip-xml-graph-digest-v1-draft","actualStoredBytesVerified":true,"fullPresentationXsd":false},"pageCoverage":document.slide_order,"previews":preview_evidence,"layoutQualityProven":false,"nativeEditabilityProven":false,"playbackProven":false,"targetApplicationProven":false}))?;
    let claims=[ClaimKind::Structure,ClaimKind::Layout,ClaimKind::NativeEditability,ClaimKind::Playback,ClaimKind::TargetApplication].into_iter().map(|kind|Claim {
        kind,status:if kind==ClaimKind::Structure {ClaimStatus::Passed}else{ClaimStatus::NotProven},
        subject_sha256:pptx_asset.sha256.clone(),basis:if kind==ClaimKind::Structure {ClaimBasis::StaticInspection}else{ClaimBasis::None},
        profile_id:if kind==ClaimKind::Structure {"opc-zip-xml-graph-digest-v1-draft".into()}else{PROFILE.into()},
        evidence_asset_ids:vec![quality.id.clone()],reason:Some(match kind {
            ClaimKind::Structure=>"Actual stored OPC/ZIP/XML graph and digest verified; full PresentationML XSD is not asserted.",
            ClaimKind::Layout=>"Every editor preview was rendered, but full layout fidelity has not been proved.",
            ClaimKind::NativeEditability=>"Native output was written; complete external editing roundtrip has not been proved.",
            ClaimKind::Playback=>"Editor previews do not prove animation, transition or media playback.",
            ClaimKind::TargetApplication=>"This delivery has no target-application test evidence.",
        }.into()),
    }).collect();
    // Package already counted in the byte budget; moving its same reader neither
    // copies bytes nor turns the calculation into publication.
    outputs.outputs.push(ProducedArtifact {
        name: "presentation".into(),
        asset: pptx_asset.clone(),
        reader: package.into_reader(),
    });
    if outputs.outputs.len() > limits.max_artifacts {
        return Err(DeliveryError::Limit("artifact count"));
    }
    let bundle = DeliveryBundle {
        version: "musteroffice.bundle/1-draft".into(),
        document: DeliveredDocument {
            document_id: document.id.clone(),
            revision: snapshot.revision.clone(),
            model_asset_id: model.id,
        },
        profile_id: PROFILE.into(),
        versions: Versions {
            engine: concat!("MusterOffice/", env!("CARGO_PKG_VERSION")).into(),
            document_schema: "musteroffice.presentation/0.1-draft".into(),
            operation_schema: "musteroffice.operations/1-draft".into(),
            rules: PROFILE.into(),
            feature_registry_sha256: registry.sha256,
            font_profile_sha256: font_profile.sha256,
        },
        pptx_asset_id: pptx_asset.id,
        assets: outputs.outputs.iter().map(|o| o.asset.clone()).collect(),
        previews,
        claims,
    };
    crate::integrity::validate(&bundle, &snapshot)?;
    cancel(check)?;
    Ok(DeliveryCandidate {
        bundle,
        artifacts: outputs.outputs,
        semantic_digest: snapshot.semantic_digest,
        settings_digest,
    })
}
