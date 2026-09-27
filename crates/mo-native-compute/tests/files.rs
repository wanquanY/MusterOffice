use mo_embedded_sdk::{common::Digest, operation::FailureCode};
use mo_native_compute::{
    FileCall, compute,
    workspace::{Directory, leaf},
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mo-file-test-{}-{epoch}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("spool")).unwrap();
        Self(path)
    }
    fn json(&self, name: &str, value: Value) {
        fs::write(self.0.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn create(&self) {
        self.json("invocation.json",json!({"request":{"contractVersion":"musteroffice.computation/1-draft","requestId":"files:create","profileId":"presentations-author-model-v01-draft","action":{"kind":"create","document":serde_json::from_str::<Value>(include_str!("../../../fixtures/presentations/basic-shape.json")).unwrap()}}}));
        self.json("inputs.json", json!([]));
    }
    fn call<'a>(&self, spool: &'a Path, output: &'a Path) -> FileCall<'a> {
        FileCall {
            invocation: File::open(self.0.join("invocation.json")).unwrap(),
            inputs: File::open(self.0.join("inputs.json")).unwrap(),
            temporary_directory: spool,
            output_directory: output,
            exporter: None,
        }
    }
    fn clean(&self) {
        let root = self.0.join("spool");
        for child in fs::read_dir(root).unwrap() {
            let child = child.unwrap();
            assert_eq!(child.file_name(), ".mo-executions-v1");
            assert_eq!(
                fs::read_dir(child.path())
                    .unwrap()
                    .map(|p| p.unwrap().file_name())
                    .collect::<Vec<_>>(),
                ["registry.lock"]
            );
        }
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn cancellation_at_each_observed_phase_removes_only_this_calls_output() {
    let root = Root::new();
    root.create();
    let spool = root.0.join("spool");
    let output = root.0.join("success");
    let polls = Cell::new(0usize);
    let result = compute(
        root.call(&spool, &output),
        &|_| panic!("no resource reads for create"),
        &|| {
            polls.set(polls.get() + 1);
            false
        },
    )
    .unwrap();
    let count = polls.get();
    assert!(count > 5);
    assert!(!result.product_committed);
    let original = fs::read(output.join("result.json")).unwrap();
    assert_eq!(
        Digest::from_sha256(Sha256::digest(&original).into()),
        result.result_sha256
    );
    for cancel_at in 0..count {
        let destination = root.0.join(format!("cancel-{cancel_at}"));
        polls.set(0);
        let error = compute(
            root.call(&spool, &destination),
            &|_| panic!("no resources"),
            &|| {
                let n = polls.get();
                polls.set(n + 1);
                n >= cancel_at
            },
        )
        .unwrap_err();
        assert_eq!(error.code, FailureCode::Cancelled);
        assert!(!destination.exists(), "cancellation point {cancel_at}");
        root.clean();
    }
    assert!(
        compute(
            root.call(&spool, &output),
            &|_| panic!("no resources"),
            &|| false
        )
        .is_err()
    );
    assert_eq!(fs::read(output.join("result.json")).unwrap(), original);
}

#[test]
fn cancellation_during_asset_copy_releases_private_staging_and_keeps_source() {
    let root = Root::new();
    let data = vec![42u8; 1024 * 1024];
    fs::write(root.0.join("source.bin"), &data).unwrap();
    root.json("invocation.json",json!({"request":{"contractVersion":"musteroffice.computation/1-draft","requestId":"files:import","profileId":"presentations-author-model-v01-draft","action":{"kind":"import","documentId":"imported","source":{"resourceId":"source","assetId":"source"}}}}));
    root.json("inputs.json",json!([{"file":"source.bin","info":{"id":"source","descriptor":{"sha256":format!("{:x}",Sha256::digest(&data)),"byteLength":data.len().to_string(),"mediaType":"application/vnd.openxmlformats-officedocument.presentationml.presentation"},"verification":"bytesSha256"}}]));
    let copying = Cell::new(false);
    let polls = Cell::new(0);
    let error = compute(
        root.call(&root.0.join("spool"), &root.0.join("output")),
        &|name| {
            copying.set(true);
            File::open(root.0.join(name))
        },
        &|| {
            if copying.get() {
                polls.set(polls.get() + 1);
                polls.get() >= 3
            } else {
                false
            }
        },
    )
    .unwrap_err();
    assert_eq!(error.code, FailureCode::Cancelled);
    assert!(copying.get());
    root.clean();
    assert!(!root.0.join("output").exists());
    assert_eq!(fs::read(root.0.join("source.bin")).unwrap(), data);
}

#[test]
fn file_bridge_rejects_paths_reserved_names_and_non_regular_inputs() {
    for name in [
        "",
        ".",
        "..",
        "../a",
        "/etc/passwd",
        "a/b",
        "a\\b",
        "C:foo",
        "a%2fb",
        "a?b",
        "NUL.txt",
        "COM1",
        "lpt9.bin",
        "a.",
        "a\n",
    ] {
        assert!(leaf(name).is_err(), "{name:?}");
    }
    let root = Root::new();
    root.create();
    let directory = Directory::open(&root.0).unwrap();
    assert!(directory.read("invocation.json").is_ok());
    assert!(directory.read("spool").is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.0.join("invocation.json"), root.0.join("link")).unwrap();
        assert!(directory.read("link").is_err());
        std::os::unix::fs::symlink(root.0.join("spool"), root.0.join("linked-dir")).unwrap();
        assert!(directory.directory("linked-dir").is_err());
    }
}
