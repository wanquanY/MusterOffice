use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("mo-resource-cli-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("host.sqlite")
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(db: &Path, args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mo-host"))
        .arg(db)
        .args(["principal", "scope"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
fn response(db: &Path, args: &[&str], input: &[u8]) -> Value {
    let output = run(db, args, input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn command(db: &Path, value: Value) -> Value {
    response(db, &[], format!("{value}\n").as_bytes())
}

#[test]
fn chunked_cli_upload_restart_seal_and_binary_range_read() {
    let dir = Directory::new();
    let db = dir.path();
    let n = mo_operation_service::ASSET_CHUNK_BYTES;
    let bytes: Vec<u8> = (0..n * 2 + 17).map(|i| (i % 251) as u8).collect();
    let request = json!({"operation":"beginUpload","request":{"requestId":"resource","descriptor":{"sha256":format!("{:x}",Sha256::digest(&bytes)),"byteLength":bytes.len().to_string(),"mediaType":"application/octet-stream"}}});
    let begin = command(&db, request.clone());
    assert_eq!(begin["outcome"], "succeeded");
    let id = begin["result"]["upload"]["id"].as_str().unwrap();
    assert_eq!(begin["result"]["upload"]["state"], "uploading");
    for (i, chunk) in bytes.chunks(n).enumerate() {
        let result = response(&db, &["append", id, &(i * n).to_string()], chunk);
        assert_eq!(result["outcome"], "succeeded");
        assert_eq!(
            result["result"]["upload"]["receivedBytes"],
            ((i * n) + chunk.len()).to_string()
        );
        assert!(result["result"]["upload"]["asset"].is_null());
    }
    let sealed = command(&db, json!({"operation":"sealUpload","uploadId":id}));
    assert_eq!(sealed["result"]["upload"]["state"], "sealed");
    assert_eq!(command(&db, request), sealed);
    let asset = &sealed["result"]["upload"]["asset"];
    assert_eq!(
        command(&db, json!({"operation":"readAsset","assetId":asset["id"]}))["result"]["asset"],
        *asset
    );
    for (offset, length) in [(0, bytes.len()), (n - 9, 40), (bytes.len(), 0)] {
        let output = run(
            &db,
            &[
                "read-asset",
                asset["id"].as_str().unwrap(),
                &offset.to_string(),
                &length.to_string(),
            ],
            b"",
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, bytes[offset..offset + length]);
        assert!(output.stderr.is_empty());
    }
    let outside = run(
        &db,
        &[
            "read-asset",
            asset["id"].as_str().unwrap(),
            &bytes.len().to_string(),
            "1",
        ],
        b"",
    );
    assert!(!outside.status.success());
    assert!(outside.stdout.is_empty());
}

#[test]
fn invalid_chunk_and_public_binary_json_do_not_allocate_or_publish_unbounded_data() {
    let dir = Directory::new();
    let db = dir.path();
    let result = command(
        &db,
        json!({"operation":"beginUpload","request":{"requestId":"bad","descriptor":{"sha256":"00".repeat(32),"byteLength":"0","mediaType":"application/octet-stream"}},"bytes":[0,1,2]}),
    );
    assert_eq!(result["error"]["code"], "INPUT_INVALID");
    let result = response(
        &db,
        &["append", "not-a-handle", "0"],
        &vec![0; mo_operation_service::ASSET_CHUNK_BYTES + 1],
    );
    assert_eq!(result["error"]["code"], "INPUT_INVALID");
}
