use serde_json::{Value, json};
#[path = "../../test-support/temporary_directory.rs"]
mod directory;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(directory::temporary_directory("mo-cli-test"))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn invoke(db: &Path, input: &str, extra: &[&str]) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mo-host"))
        .arg(db)
        .args(["agent:cli", "workspace:cli"])
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    serde_json::from_slice(&output.stdout).unwrap()
}
fn create() -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    json!({"operation":"submit","request":{"contractVersion":"musteroffice.operations/1-draft","requestId":"create","profileId":"presentations-author-model-v01-draft","outputMode":"job","action":{"kind":"create","document":fixture["page"]["document"]}}})
}
fn call(db: &Path, value: &Value) -> Value {
    invoke(db, &format!("{value}\n"), &[])
}

#[test]
fn real_cli_persists_queued_work_edits_queries_and_retry_across_processes() {
    let dir = Directory::new();
    let db = dir.0.join("operations.sqlite");
    let create = create();
    let accepted = call(&db, &create);
    assert_eq!(accepted["outcome"], "accepted");
    assert_eq!(accepted["job"]["state"], "queued");
    assert_eq!(accepted["pollAfterMs"], 1000);
    let id = accepted["job"]["id"].as_str().unwrap();
    let completed = invoke(&db, "", &["run", id]);
    assert_eq!(completed["outcome"], "succeeded");
    assert_eq!(completed["result"]["job"]["state"], "succeeded");
    let old = call(
        &db,
        &json!({"operation":"readDocument","documentId":"document:fixture","revision":null}),
    );
    let snapshot = &old["result"]["snapshot"];
    assert_eq!(
        snapshot["document"]["title"],
        "Synthetic Unicode editing fixture"
    );
    let edit = json!({"operation":"submit","request":{"contractVersion":"musteroffice.operations/1-draft","requestId":"edit","profileId":"presentations-author-model-v01-draft","outputMode":"sync","action":{"kind":"apply","documentId":"document:fixture","baseRevision":snapshot["revision"],"operations":[{"operationId":"title","operation":{"kind":"setTitle","title":"通过 Agent 跨进程编辑"}}]}}});
    let changed = call(&db, &edit);
    assert_eq!(changed["outcome"], "succeeded");
    assert_eq!(call(&db, &edit), changed);
    assert_eq!(call(&db, &create), completed);
    let current = call(
        &db,
        &json!({"operation":"readDocument","documentId":"document:fixture"}),
    );
    assert_eq!(
        current["result"]["snapshot"]["document"]["title"],
        "通过 Agent 跨进程编辑"
    );
    assert_ne!(
        current["result"]["snapshot"]["revision"],
        snapshot["revision"]
    );
    assert_eq!(
        call(
            &db,
            &json!({"operation":"readDocument","documentId":"document:fixture","revision":snapshot["revision"]})
        ),
        old
    );
    // The executable digest pins an accepted job across individual processes.
    assert_eq!(
        completed["result"]["job"]["executorDigest"],
        changed["result"]["job"]["executorDigest"]
    );
}

#[test]
fn cli_does_not_accept_authority_or_paths_inside_public_commands() {
    let dir = Directory::new();
    let db = dir.0.join("operations.sqlite");
    let mut request = create();
    request["request"]["principal"] = json!("admin");
    let result = call(&db, &request);
    assert_eq!(result["outcome"], "failed");
    assert_eq!(result["error"]["code"], "INPUT_INVALID");
    assert_eq!(
        invoke(
            &db,
            r#"{"operation":"getJob","jobId":"one","jobId":"two"}
"#,
            &[]
        )["error"]["code"],
        "INPUT_INVALID"
    );
    assert_eq!(
        call(
            &db,
            &json!({"operation":"readDocument","documentId":"document:fixture","path":"/tmp/arbitrary"})
        )["error"]["code"],
        "INPUT_INVALID"
    );
    assert_eq!(call(&db, &create())["outcome"], "accepted");
}
