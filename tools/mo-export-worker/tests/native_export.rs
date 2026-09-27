#[cfg(unix)]
#[path = "native_export/faults.rs"]
mod faults;
#[path = "native_export/recovery.rs"]
mod recovery;
mod support;
use mo_common::Digest;
use mo_native_export::NativeExportCandidate;
use mo_operation_service::{DocumentAction, FailureCode};
use mo_presentation_delivery::{AssetRole, ClaimStatus};
use std::{cell::Cell, fs, path::Path};
use support::*;

fn bytes(candidate: &NativeExportCandidate, id: &mo_common::RequestId) -> Vec<u8> {
    let source = candidate.open(id).unwrap();
    let mut bytes = vec![0; source.byte_length as usize];
    source.reader.read_exact_at(&mut bytes, 0).unwrap();
    bytes
}

#[test]
fn real_worker_retains_verified_outputs_repeats_deterministically_and_cleans() {
    let root = Root::new();
    let exporter = exporter(&root);
    let (snapshot, request, assets) = input(exporter.renderer_identity());
    let candidate = exporter
        .prepare(&request, snapshot.clone(), &assets, &|| false)
        .unwrap();
    assert_eq!(candidate.request_digest(), &request.digest().unwrap());
    assert_eq!(candidate.expectation().revision, snapshot.revision);
    assert_eq!(
        candidate.expectation().renderer,
        exporter.renderer_identity()
    );
    assert_eq!(
        candidate.expectation().semantic_digest,
        snapshot.semantic_digest
    );
    let bundle = &candidate.receipt().bundle;
    assert_eq!(bundle.assets.len(), 12);
    assert_eq!(bundle.previews.len(), 2);
    assert_eq!(
        bundle
            .claims
            .iter()
            .filter(|c| c.status == ClaimStatus::Passed)
            .count(),
        1
    );
    for asset in candidate.assets() {
        assert_eq!(digest(&bytes(&candidate, &asset.id)), asset.sha256);
    }
    assert_eq!(root.executions().len(), 1);
    assert_eq!(exporter.recover_spools(64).unwrap().live, 1);
    let repeated = exporter
        .prepare(&request, snapshot, &assets, &|| false)
        .unwrap();
    assert_eq!(
        serde_json::to_value(candidate.receipt()).unwrap(),
        serde_json::to_value(repeated.receipt()).unwrap()
    );
    for asset in candidate.assets() {
        assert_eq!(bytes(&candidate, &asset.id), bytes(&repeated, &asset.id));
    }
    assert!(
        assets
            .0
            .iter()
            .all(|(_, source)| source.max_read.get() <= 65536)
    );
    if let Some(path) = std::env::var_os("MO_NATIVE_EXPORT_EVIDENCE") {
        retain_evidence(
            Path::new(&path),
            &candidate,
            &request,
            &exporter.renderer_identity(),
        );
    }
    repeated.discard().unwrap();
    assert_eq!(root.executions().len(), 1);
    drop(candidate);
    root.clean();
}

#[test]
fn corrupt_asset_snapshot_and_revision_fail_without_retained_outputs() {
    let root = Root::new();
    let exporter = exporter(&root);
    for fault in 0..4 {
        let (mut snapshot, mut request, mut assets) = input(exporter.renderer_identity());
        match fault {
            0 => assets.0[0].1.bytes[0] ^= 1,
            1 => snapshot.semantic_digest = Digest::from_sha256([0; 32]),
            2 => snapshot.revision = Digest::from_sha256([0; 32]),
            3 => {
                let DocumentAction::Export { settings, .. } = &mut request.action else {
                    unreachable!()
                };
                settings.renderer.implementation_sha256 = Digest::from_sha256([0; 32]);
            }
            _ => unreachable!(),
        }
        let error = exporter
            .prepare(&request, snapshot, &assets, &|| false)
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            [
                FailureCode::ResourceConflict,
                FailureCode::InputInvalid,
                FailureCode::RevisionConflict,
                FailureCode::ExecutorMismatch
            ][fault],
            "{error:?}"
        );
        root.clean();
    }
}

#[test]
fn cancellation_before_start_and_during_output_removes_all_execution_files() {
    let root = Root::new();
    let exporter = exporter(&root);
    for during in [false, true] {
        let (snapshot, request, assets) = input(exporter.renderer_identity());
        let triggered = Cell::new(false);
        let check = || {
            let ready = !during
                || root.executions().into_iter().any(|execution| {
                    // participants.lock is infrastructure, not output.
                    fs::read_dir(execution).is_ok_and(|children| children.count() >= 4)
                });
            if ready {
                triggered.set(true);
            }
            triggered.get()
        };
        let error = exporter
            .prepare(&request, snapshot, &assets, &check)
            .err()
            .unwrap();
        assert_eq!(error.code, FailureCode::Cancelled);
        assert!(triggered.get());
        root.clean();
    }
}

#[test]
fn input_authorization_and_reader_failures_never_become_candidates() {
    let root = Root::new();
    let exporter = exporter(&root);
    for failure in [false, true] {
        let (snapshot, request, mut assets) = input(exporter.renderer_identity());
        if failure {
            assets.0[0].1.fail = true;
        } else {
            assets.0.pop();
        }
        let error = exporter
            .prepare(&request, snapshot, &assets, &|| false)
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            if failure {
                FailureCode::ExecutionInterrupted
            } else {
                FailureCode::NotAuthorized
            }
        );
        root.clean();
    }
}

#[test]
fn native_font_failure_keeps_shared_structured_diagnostic() {
    let root = Root::new();
    let exporter = exporter(&root);
    let (snapshot, mut request, assets) = input(exporter.renderer_identity());
    let DocumentAction::Export { settings, .. } = &mut request.action else {
        unreachable!()
    };
    settings.delivery.fonts.as_mut().unwrap().fonts[0].expected_sha256 =
        Digest::from_sha256([0; 32]);
    let error = exporter
        .prepare(&request, snapshot, &assets, &|| false)
        .err()
        .unwrap();
    assert_eq!(error.code, FailureCode::RenderFailure);
    let detail = error.detail.unwrap();
    assert_eq!(detail["stage"], "fonts");
    assert_eq!(detail["error"]["code"], "RESOURCE_CONFLICT");
    root.clean();
}

fn retain_evidence(
    path: &Path,
    candidate: &NativeExportCandidate,
    request: &mo_operation_service::OperationRequest,
    renderer: &mo_presentation_delivery::RendererIdentity,
) {
    fs::create_dir(path).unwrap();
    let bundle = &candidate.receipt().bundle;
    let mut files = vec![];
    let mut evidence = 0;
    let mut images = 0;
    for (i, asset) in candidate.assets().iter().enumerate() {
        let data = bytes(candidate, &asset.id);
        let name = match asset.role {
            AssetRole::Pptx => "presentation".to_owned(),
            AssetRole::Preview => format!(
                "preview:{}",
                bundle
                    .previews
                    .iter()
                    .position(|p| p.image_asset_id == asset.id)
                    .unwrap()
            ),
            AssetRole::Image => {
                let name = format!("resource:{images}");
                images += 1;
                name
            }
            _ if asset.id == bundle.document.model_asset_id => "model".to_owned(),
            _ if !asset.media_type.ends_with("json") => "font-bundle".to_owned(),
            _ => {
                let json: serde_json::Value = serde_json::from_slice(&data).unwrap();
                if json.get("previewRenderer").is_some() {
                    "delivery-context".into()
                } else if json.get("fullPresentationCapability").is_some() {
                    "feature-registry".into()
                } else if json.get("manifest").is_some() {
                    "font-profile".into()
                } else if json.get("pageCoverage").is_some() {
                    "quality".into()
                } else {
                    let name = format!("preview-evidence:{evidence}");
                    evidence += 1;
                    name
                }
            }
        };
        let extension = match asset.role {
            AssetRole::Pptx => "pptx",
            AssetRole::Preview | AssetRole::Image => "png",
            _ if asset.media_type.ends_with("json") => "json",
            _ => "bin",
        };
        let file = format!("{i:03}.{extension}");
        fs::write(path.join(&file), data).unwrap();
        files.push(serde_json::json!({"name": name, "file": file, "asset": asset}));
    }
    for (name, value) in [
        ("bundle", serde_json::to_value(bundle).unwrap()),
        ("files", serde_json::to_value(files).unwrap()),
        (
            "receipt",
            serde_json::to_value(candidate.receipt()).unwrap(),
        ),
        (
            "inspection",
            serde_json::to_value(candidate.inspection()).unwrap(),
        ),
        ("request", serde_json::to_value(request).unwrap()),
        ("renderer", serde_json::to_value(renderer).unwrap()),
    ] {
        fs::write(
            path.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
}
