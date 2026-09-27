use mo_embedded_sdk::{
    Presentation,
    common::{ByteLength, Digest, DocumentId, OperationId, RequestId, ResourceId},
    edit::{Operation, OperationEntry, SnapshotRecord},
    operation::*,
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "mo-compute-cli-{}-{time}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("spool")).unwrap();
        Self(root)
    }
    fn json(&self, name: &str, value: &impl serde::Serialize) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
    fn run(
        &self,
        invocation: &Invocation,
        bindings: Value,
        dest: &str,
        worker: Option<(&Path, &str)>,
    ) -> Output {
        let request = self.json("invocation.json", invocation);
        let inputs = self.json("inputs.json", &bindings);
        let mut command = Command::new(env!("CARGO_BIN_EXE_mo-cli"));
        command
            .arg("compute")
            .arg(request)
            .arg(inputs)
            .arg(self.0.join("spool"))
            .arg(self.0.join(dest));
        if let Some((worker, digest)) = worker {
            command.arg(worker).arg(digest);
        }
        command.output().unwrap()
    }
    fn result(&self, name: &str, output: &Output) -> ComputationReceipt {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
        let bytes = fs::read(self.0.join(name).join("result.json")).unwrap();
        assert_eq!(summary["resultSha256"], json!(digest(&bytes)));
        assert_eq!(summary["resultByteLength"], bytes.len().to_string());
        assert_eq!(summary["productCommitted"], false);
        serde_json::from_slice(&bytes).unwrap()
    }
    fn clean_spool(&self) {
        for child in fs::read_dir(self.0.join("spool")).unwrap() {
            let child = child.unwrap();
            assert_eq!(child.file_name(), ".mo-executions-v1");
            let names: Vec<_> = fs::read_dir(child.path())
                .unwrap()
                .map(|p| p.unwrap().file_name())
                .collect();
            assert_eq!(names, ["registry.lock"]);
        }
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn digest(bytes: &[u8]) -> Digest {
    Digest::from_sha256(Sha256::digest(bytes).into())
}
fn create() -> Invocation {
    Invocation {
        request: OperationRequest {
            contract_version: ContractVersion::V1,
            request_id: RequestId::new("create").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            action: DocumentAction::Create {
                document: Box::new(
                    serde_json::from_str(include_str!(
                        "../../../fixtures/presentations/basic-shape.json"
                    ))
                    .unwrap(),
                ),
            },
        },
        snapshot: None,
    }
}
fn snapshot(receipt: ComputationReceipt) -> Box<SnapshotRecord> {
    let ComputationResult::Mutated { snapshot, .. } = receipt.result else {
        panic!("mutated")
    };
    snapshot
}
#[test]
fn create_and_edit_use_caller_files_and_never_overwrite_or_publish_failed_work() {
    let root = Root::new();
    let create = create();
    let first = root.run(&create, json!([]), "created", None);
    let original = snapshot(root.result("created", &first));
    let edit = Invocation {
        request: OperationRequest {
            request_id: RequestId::new("edit").unwrap(),
            action: DocumentAction::Apply {
                document_id: original.document.id.clone(),
                base_revision: original.revision.clone(),
                operations: vec![OperationEntry {
                    operation_id: OperationId::new("title").unwrap(),
                    operation: Operation::SetTitle {
                        title: "direct CLI".into(),
                    },
                }],
            },
            ..create.request.clone()
        },
        snapshot: Some(original.clone()),
    };
    let edited = root.run(&edit, json!([]), "edited", None);
    let latest = snapshot(root.result("edited", &edited));
    assert_eq!(latest.document.title, "direct CLI");
    assert_ne!(latest.revision, original.revision);
    assert_eq!(
        root.result("created", &first).request_digest,
        create.request.digest().unwrap()
    );
    let protected = fs::read(root.0.join("created/result.json")).unwrap();
    let exists = root.run(&create, json!([]), "created", None);
    assert!(!exists.status.success());
    assert!(exists.stdout.is_empty());
    assert_eq!(
        fs::read(root.0.join("created/result.json")).unwrap(),
        protected
    );
    let mut stale = edit;
    stale.snapshot = Some(latest);
    let failed = root.run(&stale, json!([]), "failed", None);
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert!(!root.0.join("failed").exists());
    let diagnostic: Value = serde_json::from_slice(&failed.stderr).unwrap();
    assert_eq!(diagnostic["outcome"], "failed");
    assert_eq!(diagnostic["error"]["code"], "REVISION_CONFLICT");
    let mut invalid = create;
    invalid.snapshot = Some(original);
    let failed = root.run(&invalid, json!([]), "invalid", None);
    assert!(!failed.status.success());
    assert!(!root.0.join("invalid").exists());
    root.clean_spool();
}
fn binding(root: &Root, id: &str, name: &str, media: &str, bytes: &[u8]) -> Value {
    fs::write(root.0.join(name), bytes).unwrap();
    json!({"file":name,"info":AssetInfo{id:AssetId::new(id).unwrap(),descriptor:AssetDescriptor{sha256:digest(bytes),byte_length:ByteLength::new(bytes.len() as u64),media_type:media.into()},verification:AssetVerification::BytesSha256}})
}
#[test]
fn import_checks_actual_source_bytes_and_cleans_temporary_inputs() {
    let root = Root::new();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/presentations/native-export");
    let generated = Command::new(env!("CARGO_BIN_EXE_mo-cli"))
        .arg("pptx-export")
        .arg(fixture.join("request.json"))
        .arg(fixture.join("resources.bin"))
        .arg(root.0.join("source.pptx"))
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let bytes = fs::read(root.0.join("source.pptx")).unwrap();
    let source = binding(
        &root,
        "source",
        "input.pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        &bytes,
    );
    let invocation = Invocation {
        request: OperationRequest {
            action: DocumentAction::Import {
                document_id: DocumentId::new("imported").unwrap(),
                source: AssetBinding {
                    resource_id: ResourceId::new("source").unwrap(),
                    asset_id: AssetId::new("source").unwrap(),
                },
            },
            ..create().request
        },
        snapshot: None,
    };
    let imported = root.run(&invocation, json!([source]), "imported", None);
    let snapshot = snapshot(root.result("imported", &imported));
    assert!(snapshot.document.source_bindings.is_some());
    assert_eq!(snapshot.document.slide_order.len(), 2);
    root.clean_spool();
    let mut corrupt = bytes.clone();
    corrupt[0] ^= 1;
    fs::write(root.0.join("input.pptx"), corrupt).unwrap();
    for (name, bindings) in [
        ("bad-digest", json!([source])),
        ("duplicate", json!([source, source])),
        ("missing", json!([])),
    ] {
        let failed = root.run(&invocation, bindings, name, None);
        assert!(!failed.status.success());
        assert!(failed.stdout.is_empty());
        assert!(!root.0.join(name).exists());
        root.clean_spool();
    }
    let mut oversized = source.clone();
    oversized["info"]["descriptor"]["byteLength"] = "536870913".into();
    assert!(
        !root
            .run(&invocation, json!([oversized]), "over-budget", None)
            .status
            .success()
    );
    root.clean_spool();
}

#[test]
#[ignore = "requires a pinned actual export worker; explicitly run by native verification"]
fn direct_cli_exports_verified_pptx_and_previews_with_no_database() {
    let root = Root::new();
    let worker = PathBuf::from(std::env::var_os("MO_EXPORT_WORKER_TEST_BIN").expect("worker"));
    let pin = std::env::var("MO_EXPORT_WORKER_TEST_SHA256").expect("worker hash");
    assert_eq!(digest(&fs::read(&worker).unwrap()).as_str(), pin);
    let value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let original = Presentation::create(
        serde_json::from_value(value["document"].clone()).unwrap(),
        &|| false,
    )
    .unwrap()
    .into_snapshot();
    let image = binding(
        &root,
        "image:1",
        "image.bin",
        "image/png",
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    );
    let font = binding(
        &root,
        "font:1",
        "font.bin",
        "font/ttf",
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
    );
    let request:OperationRequest=serde_json::from_value(json!({
        "contractVersion":"musteroffice.computation/1-draft","requestId":"export:embedded-test","profileId":"presentations-pptx-resource-delivery-v1-draft",
        "action":{"kind":"export","documentId":original.document.id,"baseRevision":original.revision,
        "settings":{"delivery":value["settings"],"resources":[{"resourceId":"resource:checker","assetId":"image:1"}],"fontAssetId":"font:1",
        "renderer":{"implementationSha256":pin,"profile":"drawingml-resource-page-q32-v1-draft"}}}})).unwrap();
    let invocation = Invocation {
        request,
        snapshot: Some(Box::new(original)),
    };
    let output = root.run(
        &invocation,
        json!([image, font]),
        "exported",
        Some((&worker, &pin)),
    );
    let result = root.result("exported", &output);
    let ComputationResult::Exported { receipt } = result.result else {
        panic!("exported")
    };
    assert_eq!(receipt.bundle.assets.len(), 12);
    assert_eq!(receipt.bundle.previews.len(), 2);
    let files: Vec<Value> =
        serde_json::from_slice(&fs::read(root.0.join("exported/files.json")).unwrap()).unwrap();
    for file in files {
        let bytes = fs::read(root.0.join("exported").join(file["file"].as_str().unwrap())).unwrap();
        assert_eq!(file["asset"]["sha256"], json!(digest(&bytes)));
        assert_eq!(file["asset"]["byteLength"], bytes.len().to_string());
    }
    root.clean_spool();
    let mut bad = invocation;
    bad.snapshot.as_mut().unwrap().document.title.push('!');
    let failed = root.run(
        &bad,
        json!([image, font]),
        "tampered",
        Some((&worker, &pin)),
    );
    assert!(!failed.status.success());
    assert!(!root.0.join("tampered").exists());
    root.clean_spool();
}
