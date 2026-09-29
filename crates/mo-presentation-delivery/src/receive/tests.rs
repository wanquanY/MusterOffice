use super::*;
use serde_json::{Value, json};
use std::cell::Cell;

struct Source {
    bytes: BTreeMap<RequestId, Vec<u8>>,
    opens: Cell<usize>,
}
impl DeliverySource for Source {
    fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError> {
        self.opens.set(self.opens.get() + 1);
        let bytes = self
            .bytes
            .get(id)
            .ok_or(DeliveryError::Invalid("fixture missing asset"))?;
        Ok(Content {
            reader: bytes,
            byte_length: bytes.len() as u64,
        })
    }
}
struct Fixture {
    bundle: DeliveryBundle,
    expected: DeliveryExpectation,
    source: Source,
}
impl Fixture {
    fn new() -> Self {
        let value: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/presentations/delivery-receive/request.json"
        ))
        .unwrap();
        let bytes =
            include_bytes!("../../../../fixtures/presentations/delivery-receive/assets.bin");
        let mut content = BTreeMap::new();
        for range in value["contents"].as_array().unwrap() {
            let start = range["byteOffset"]
                .as_str()
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let length = range["byteLength"]
                .as_str()
                .unwrap()
                .parse::<usize>()
                .unwrap();
            content.insert(
                serde_json::from_value(range["assetId"].clone()).unwrap(),
                bytes[start..start + length].to_vec(),
            );
        }
        Self {
            bundle: serde_json::from_value(value["bundle"].clone()).unwrap(),
            expected: serde_json::from_value(value["expected"].clone()).unwrap(),
            source: Source {
                bytes: content,
                opens: Cell::new(0),
            },
        }
    }
    fn run(&self) -> Result<ReceivedDelivery, DeliveryError> {
        inspect(
            &self.bundle,
            &self.expected,
            &self.source,
            DeliveryLimits::default(),
            &|| false,
        )
    }
    fn id(&self, mime: &str) -> RequestId {
        self.bundle
            .assets
            .iter()
            .find(|a| a.media_type == mime)
            .unwrap()
            .id
            .clone()
    }
    fn bytes(&mut self, id: &RequestId, data: Vec<u8>) {
        let a = self.bundle.assets.iter_mut().find(|a| a.id == *id).unwrap();
        a.sha256 = artifact::digest(&data, data.len() as u64, &|| false).unwrap();
        a.byte_length = ByteLength::new(data.len() as u64);
        self.source.bytes.insert(id.clone(), data);
    }
    fn json(&mut self, id: &RequestId, edit: impl FnOnce(&mut Value)) {
        let mut value = serde_json::from_slice(&self.source.bytes[id]).unwrap();
        edit(&mut value);
        self.bytes(id, serde_json::to_vec(&value).unwrap());
    }
    fn rejects(&self, fragment: &str) {
        let error = self.run().err().expect("must reject").to_string();
        assert!(error.contains(fragment), "{fragment}: {error}");
    }
}
#[test]
fn owned_native_delivery_is_received_with_original_claims() {
    let f = Fixture::new();
    let received = f.run().unwrap();
    let report = received.report();
    assert_eq!(report.assets_verified, 12);
    assert_eq!(report.pages, 2);
    let layout = report.layout_diagnostics.as_ref().unwrap();
    assert_eq!(layout.unmeasured_pages, 2);
    assert_eq!(layout.measured_pages, 0);
    assert!(layout.findings.is_empty());
    assert_eq!(received.preview_measurements().len(), report.pages);
    for (measurement, page) in received
        .preview_measurements()
        .iter()
        .zip(&f.bundle.previews)
    {
        assert_eq!(measurement.page_id, page.page_id);
        assert!(measurement.text_capacity.is_none());
        assert!(
            f.bundle
                .assets
                .iter()
                .any(|asset| asset.id == measurement.evidence_asset_id
                    && asset.role == AssetRole::QualityReport
                    && asset.media_type == "application/json")
        );
    }
    assert_eq!(report.total_bytes.get(), 69623);
    assert_eq!(
        report.bundle_digest.to_string(),
        "52cbba2e73dcbaee03377ce82c5c6a809d65f44454f8490205ae9b6e8044087d"
    );
    assert_eq!(
        serde_json::to_value(&report.declared_claims).unwrap(),
        serde_json::to_value(&f.bundle.claims).unwrap()
    );
    assert_eq!(received.snapshot().document.title, "SDK real integration");
}

#[test]
fn layout_object_addresses_bind_authored_and_retained_model_identities() {
    let f = Fixture::new();
    let snapshot: SnapshotRecord =
        serde_json::from_slice(&f.source.bytes[&f.bundle.document.model_asset_id]).unwrap();
    let context: Value = serde_json::from_slice(&f.source.bytes[&f.id(CONTEXT_MIME)]).unwrap();
    let settings: DeliverySettings = serde_json::from_value(context["settings"].clone()).unwrap();
    let author = mo_pptx::AuthorPlan::new(
        &snapshot.document,
        &settings.defaults,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let expected_ids = author.bindings().object_ids.clone();
    let authored =
        diagnostics::object_bindings(&mo_pptx::PresentationPlan::Author(author), &|| false)
            .unwrap();
    assert_eq!(authored.len(), snapshot.document.objects.len());
    for ((part, native), id) in &authored {
        assert!(part.starts_with("/ppt/"));
        assert_eq!(*native, expected_ids[id]);
    }
    // Reimport the same actual file. Native addresses stay stable, while the
    // editable model IDs become source-bound IDs. Object names are not used.
    let bytes = &f.source.bytes[&f.bundle.pptx_asset_id];
    let reader: &dyn mo_opc::ReaderAt = bytes;
    let source = Package::open(reader, bytes.len() as u64, Default::default(), &|| false).unwrap();
    let imported = mo_pptx::source::document::import_document(
        &source,
        DocumentId::new("document:imported-layout").unwrap(),
        mo_common::ResourceId::new("resource:source").unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let retained =
        mo_pptx::source::document::SourcePlan::new(&imported, &source, Default::default(), &|| {
            false
        })
        .unwrap();
    let retained = diagnostics::object_bindings(
        &mo_pptx::PresentationPlan::Retained {
            plan: Box::new(retained),
            source,
        },
        &|| false,
    )
    .unwrap();
    let bindings = imported.source_bindings.as_ref().unwrap();
    assert_eq!(retained.len(), bindings.objects.len());
    for (id, binding) in &bindings.objects {
        assert_eq!(retained[&(binding.part.clone(), binding.native_id)], *id);
    }
    for address in authored.keys() {
        assert!(retained.contains_key(address));
        assert_ne!(retained[address], authored[address]);
    }
}

#[test]
fn playback_inputs_reuse_verified_fonts_geometry_and_actual_page_order() {
    let f = Fixture::new();
    let received = f.run().unwrap();
    let opened = f.source.opens.get();
    let claims = serde_json::to_value(&received.report().declared_claims).unwrap();
    let inputs = received.playback_inputs(641, &|| false).unwrap();
    assert_eq!(f.source.opens.get(), opened, "must not reopen any asset");
    assert_eq!(inputs.revision, f.expected.revision);
    assert_eq!(inputs.document_id, f.expected.document_id);
    assert_eq!(inputs.source.id, f.bundle.pptx_asset_id);
    assert_eq!(inputs.source.role, AssetRole::Pptx);
    assert_eq!(inputs.pages.len(), 2);
    assert_eq!(inputs.pages[0].page_id, f.bundle.previews[0].page_id);
    assert_eq!(inputs.pages[1].page_id, f.bundle.previews[1].page_id);
    for (i, page) in inputs.pages.iter().enumerate() {
        assert_eq!(
            page.request.page.slide,
            format!("/ppt/slides/slide{}.xml", i + 1)
        );
        assert_eq!(
            page.request.page.expected_source_sha256,
            inputs.source.sha256
        );
        assert_eq!(page.request.page.viewport.width, 641);
        assert_eq!(page.request.page.viewport.height, 361);
    }
    let ctx: Value = serde_json::from_slice(&f.source.bytes[&f.id(CONTEXT_MIME)]).unwrap();
    assert_eq!(
        serde_json::to_value(&inputs.fonts).unwrap(),
        ctx["settings"]["fonts"]
    );
    let font = inputs.font_bundle.unwrap();
    assert_eq!(
        font.id.to_string(),
        ctx["fontBundleAssetId"].as_str().unwrap()
    );
    assert_eq!(
        font.sha256,
        artifact::digest(&f.source.bytes[&font.id], font.byte_length.get(), &|| false).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&received.report().declared_claims).unwrap(),
        claims
    );
    assert!(received.playback_inputs(0, &|| false).is_err());
    assert!(received.playback_inputs(8193, &|| false).is_err());
    assert!(matches!(
        received.playback_inputs(640, &|| true),
        Err(DeliveryError::Cancelled)
    ));
    assert_eq!(f.source.opens.get(), opened);
}

#[test]
fn optional_capacity_evidence_preserves_history_but_rejects_false_coverage() {
    let mut f = Fixture::new();
    let evidence = f
        .bundle
        .assets
        .iter()
        .find(|a| a.role == AssetRole::QualityReport && a.media_type == "application/json")
        .unwrap()
        .id
        .clone();
    let original: Value = serde_json::from_slice(&f.source.bytes[&evidence]).unwrap();
    assert!(original["render"].get("textCapacity").is_none());
    assert!(original["render"]["textFrames"].as_u64().unwrap() > 0);
    f.run().unwrap(); // Historical missing measurements retain original claims.
    f.json(&evidence, |v| {
        v["render"]["textCapacity"] = json!({
            "profile":"drawingml-text-capacity-q32-v1-draft", "frames":[],
        })
    });
    f.rejects("preview text capacity");
}

#[test]
fn direct_preview_evidence_binds_semantic_plan_separately_from_file_bytes() {
    let mut f = Fixture::new();
    let snapshot: SnapshotRecord =
        serde_json::from_slice(&f.source.bytes[&f.bundle.document.model_asset_id]).unwrap();
    let context_id = f.id(CONTEXT_MIME);
    let context: Value = serde_json::from_slice(&f.source.bytes[&context_id]).unwrap();
    let settings: DeliverySettings = serde_json::from_value(context["settings"].clone()).unwrap();
    let plan = mo_pptx::AuthorPlan::new(
        &snapshot.document,
        &settings.defaults,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let evidence: Vec<_> = f
        .bundle
        .assets
        .iter()
        .filter(|a| a.role == AssetRole::QualityReport && a.media_type == "application/json")
        .map(|a| a.id.clone())
        .collect();
    assert!(!evidence.is_empty());
    for id in &evidence {
        f.json(id, |v| {
            v["format"] = json!("musteroffice.preview-evidence/2-draft");
            v["authorPlanSha256"] = json!(plan.identity());
            v["render"]["page"]["page"]["sourceSha256"] = json!(plan.identity());
        });
    }
    f.run().unwrap();
    let pptx_sha = f
        .bundle
        .assets
        .iter()
        .find(|a| a.id == f.bundle.pptx_asset_id)
        .unwrap()
        .sha256
        .clone();
    assert_ne!(&pptx_sha, plan.identity());
    f.json(&evidence[0], |v| v["authorPlanSha256"] = json!(pptx_sha));
    f.rejects("preview evidence binding");
    f.json(&evidence[0], |v| {
        v["authorPlanSha256"] = json!(plan.identity());
        v["render"]["page"]["page"]["sourceSha256"] = json!(pptx_sha);
    });
    f.rejects("preview evidence binding");
}
#[test]
fn declared_budget_and_accepted_pins_fail_before_opening_assets() {
    let mut f = Fixture::new();
    let limits = DeliveryLimits {
        max_total_bytes: 69622,
        ..Default::default()
    };
    assert!(matches!(
        inspect(&f.bundle, &f.expected, &f.source, limits, &|| false),
        Err(DeliveryError::Limit(_))
    ));
    assert_eq!(f.source.opens.get(), 0);
    f.expected.revision = Digest::from_sha256([0; 32]);
    f.rejects("accepted operation pin");
    assert_eq!(f.source.opens.get(), 0);
}
#[test]
fn duplicate_and_absent_assets_never_become_partial_success() {
    let mut f = Fixture::new();
    f.bundle.assets.push(f.bundle.assets[0].clone());
    f.rejects("duplicate delivery asset");
    assert_eq!(f.source.opens.get(), 0);
    let mut f = Fixture::new();
    f.source.bytes.remove(&f.bundle.pptx_asset_id);
    f.rejects("fixture missing asset");
}
#[test]
fn actual_bytes_and_lengths_are_required_even_with_valid_metadata() {
    let mut f = Fixture::new();
    let id = f.id("application/octet-stream");
    f.source.bytes.get_mut(&id).unwrap()[0] ^= 1;
    f.rejects("actual delivery bytes");
    let mut f = Fixture::new();
    f.source.bytes.get_mut(&id).unwrap().pop();
    f.rejects("actual delivery bytes");
}
#[test]
fn model_semantic_digest_and_external_semantic_pin_are_independent() {
    let mut f = Fixture::new();
    let id = f.bundle.document.model_asset_id.clone();
    f.json(&id, |v| v["document"]["title"] = json!("tampered"));
    f.rejects("model semantic digest");
    let mut f = Fixture::new();
    f.expected.semantic_digest = Digest::from_sha256([0; 32]);
    f.rejects("accepted model semantic digest");
}
#[test]
fn malformed_or_duplicate_field_json_is_not_a_typed_record() {
    let mut f = Fixture::new();
    let id = f.id(CONTEXT_MIME);
    let original = &f.source.bytes[&id];
    let mut bytes = b"{\"format\":\"duplicate\",".to_vec();
    bytes.extend_from_slice(&original[1..]);
    f.bytes(&id, bytes);
    f.rejects("delivery JSON schema");
}
#[test]
fn context_settings_and_renderer_must_match_the_accepted_operation() {
    let mut f = Fixture::new();
    let id = f.id(CONTEXT_MIME);
    f.json(&id, |v| {
        v["previewRenderer"]["implementationSha256"] = json!("0".repeat(64))
    });
    f.rejects("context binding");
    let mut f = Fixture::new();
    f.json(&id, |v| v["settings"]["previewWidth"] = json!(320));
    f.rejects("accepted delivery settings digest");
}
#[test]
fn missing_or_wrong_model_resources_are_rejected() {
    let mut f = Fixture::new();
    let id = f.id(CONTEXT_MIME);
    f.json(&id, |v| v["resourceAssets"] = json!({}));
    f.rejects("context binding");
    let mut f = Fixture::new();
    let font = f.id("application/octet-stream");
    f.json(&id, |v| {
        v["resourceAssets"]["resource:checker"] = json!(font)
    });
    f.rejects("role or media type");
}
#[test]
fn font_context_and_registry_cannot_be_relabelled() {
    let mut f = Fixture::new();
    let id = f.id(CONTEXT_MIME);
    f.json(&id, |v| v["fontBundleAssetId"] = Value::Null);
    f.rejects("font context binding");
    let mut f = Fixture::new();
    let id = f.id(crate::registry::MIME);
    f.json(&id, |v| v["fullPresentationCapability"] = json!(true));
    let sha = f
        .bundle
        .assets
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .sha256
        .clone();
    f.bundle.versions.feature_registry_sha256 = sha;
    f.rejects("delivery registry version");
}
#[test]
fn extra_assets_and_swapped_page_order_do_not_pass_closure() {
    let mut f = Fixture::new();
    let mut extra = f.bundle.assets[0].clone();
    extra.id = RequestId::new("extra").unwrap();
    f.source.bytes.insert(
        extra.id.clone(),
        f.source.bytes[&f.bundle.document.model_asset_id].clone(),
    );
    f.bundle.assets.push(extra);
    f.rejects("unreferenced delivery asset");
    let mut f = Fixture::new();
    f.bundle.previews.swap(0, 1);
    f.rejects("reference closure");
}
#[test]
fn quality_and_claims_cannot_promote_unproved_capabilities() {
    let mut f = Fixture::new();
    f.bundle.claims[4].status = ClaimStatus::Passed;
    f.bundle.claims[4].basis = ClaimBasis::ApplicationTest;
    f.rejects("supported evidence profile");
    let mut f = Fixture::new();
    let id = f.id("application/vnd.musteroffice.quality+json");
    f.json(&id, |v| v["targetApplicationProven"] = json!(true));
    f.rejects("unsupported assertion");
}
#[test]
fn evidence_cannot_be_reassigned_to_a_different_page_or_pixel_digest() {
    let mut f = Fixture::new();
    let id = f.id("application/json");
    f.json(&id, |v| {
        v["render"]["page"]["page"]["slide"] = json!("/ppt/slides/slide2.xml")
    });
    f.rejects("preview evidence binding");
    let mut f = Fixture::new();
    f.json(&id, |v| {
        v["render"]["page"]["scene"]["raster"]["sha256"] = json!("0".repeat(64))
    });
    f.rejects("PNG profile or pixel digest");
}
#[test]
fn hash_valid_non_pptx_content_still_fails_package_inspection() {
    let mut f = Fixture::new();
    let id = f.bundle.pptx_asset_id.clone();
    f.bytes(&id, b"not a package".to_vec());
    let hash = f
        .bundle
        .assets
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .sha256
        .clone();
    for claim in &mut f.bundle.claims {
        claim.subject_sha256 = hash.clone();
    }
    assert!(matches!(
        f.run(),
        Err(DeliveryError::Pptx(mo_pptx::PptxError::Opc(_)))
    ));
}
#[test]
fn cancellation_remains_observable_through_reads_and_package_work() {
    let f = Fixture::new();
    let calls = Cell::new(0);
    let check = || {
        calls.set(calls.get() + 1);
        false
    };
    inspect(
        &f.bundle,
        &f.expected,
        &f.source,
        DeliveryLimits::default(),
        &check,
    )
    .unwrap();
    let checkpoints = calls.get();
    assert!(checkpoints > 700);
    for stop in [1, 20, 80, checkpoints / 2, checkpoints - 10, checkpoints] {
        calls.set(0);
        let check = || {
            calls.set(calls.get() + 1);
            calls.get() >= stop
        };
        let error = inspect(
            &f.bundle,
            &f.expected,
            &f.source,
            DeliveryLimits::default(),
            &check,
        )
        .err()
        .unwrap();
        assert!(
            error.to_string().contains("cancelled"),
            "stop {stop}: {error}"
        );
    }
}
