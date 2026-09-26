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
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "mo-cli-stream-{}-{epoch}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn entries(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect()
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/presentations/native-export")
        .join(name)
}
fn run(request: &Path, resources: &Path, dest: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mo-cli"))
        .arg("pptx-export")
        .arg(request)
        .arg(resources)
        .arg(dest)
        .output()
        .unwrap()
}

#[test]
fn cli_stream_export_keeps_exact_native_bytes_and_no_staging_leftovers() {
    let root = Root::new();
    let dest = root.0.join("slides.pptx");
    let output = run(&fixture("request.json"), &fixture("resources.bin"), &dest);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "inspected");
    let bytes = fs::read(&dest).unwrap();
    // Writer now emits Office-consumed matching-level defaults.
    assert_eq!(bytes.len(), 10962);
    let hash = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(
        hash,
        "7e41967032bfbd9ca2b174d7183f3f5917339776105262a959eaf885af6c693e"
    );
    assert_eq!(response["report"]["sha256"], hash);
    assert_eq!(root.entries(), vec![dest.clone()]);
    let second = run(&fixture("request.json"), &fixture("resources.bin"), &dest);
    assert!(!second.status.success());
    assert!(second.stdout.is_empty());
    assert_eq!(fs::read(&dest).unwrap(), bytes);
    assert_eq!(root.entries(), vec![dest]);
}

#[test]
fn cli_stream_export_rejects_damaged_resources_without_publishing() {
    let root = Root::new();
    let bundle = root.0.join("bad.bin");
    let dest = root.0.join("slides.pptx");
    let mut bytes = fs::read(fixture("resources.bin")).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(&bundle, bytes).unwrap();
    let output = run(&fixture("request.json"), &bundle, &dest);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!dest.exists());
    assert_eq!(root.entries(), vec![bundle]);
}

#[test]
fn cli_stream_export_checks_input_budget_and_schema_before_publication() {
    let root = Root::new();
    let bundle = root.0.join("oversize.bin");
    let dest = root.0.join("slides.pptx");
    fs::File::create(&bundle)
        .unwrap()
        .set_len(mo_kernel_api::MAX_INLINE_RESOURCE_BYTES as u64 + 1)
        .unwrap();
    let output = run(&fixture("request.json"), &bundle, &dest);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!dest.exists());
    assert_eq!(root.entries(), vec![bundle]);
    let request = root.0.join("invalid.json");
    fs::write(&request, b"{}").unwrap();
    let output = run(&request, &fixture("resources.bin"), &dest);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!dest.exists());
    assert!(!root.entries().iter().any(|p| {
        p.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("mo-spool-")
    }));
}
