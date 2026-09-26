use super::*;
use mo_common::Digest;
use mo_standard_host::{HostLimits, RuntimeOptions, StandardHostConfig};
use std::time::{Duration, Instant};
#[path = "../../../../crates/mo-standard-host/tests/support/mod.rs"]
mod support;

#[tokio::test]
async fn abandoned_waiter_keeps_execution_permit_and_durable_business_work() {
    let directory = support::temporary_directory("mo-mcp-abandoned-wait");
    let path = directory.join("host.sqlite");
    let runtime = NativeRuntime::start(
        StandardHostConfig::new(
            path.clone(),
            Digest::from_sha256([7; 32]),
            HostLimits::default(),
        ),
        CallContext {
            principal: PrincipalId::new("verifier").unwrap(),
            scope: ScopeId::new("fixture").unwrap(),
            permissions: [
                Permission::Create,
                Permission::ReadJob,
                Permission::ReadDocument,
            ]
            .into_iter()
            .collect(),
        },
        RuntimeOptions {
            idle_poll: Duration::from_millis(10),
            ..RuntimeOptions::default()
        },
        Arc::new(crate::config::now),
    )
    .unwrap();
    let bridge = Arc::new(HostBridge::new(runtime, 1, 1).unwrap());
    let sql = rusqlite::Connection::open(path).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let request: HostRequest =
        serde_json::from_value(serde_json::json!({"operation":"submit", "request":{
            "contractVersion":"musteroffice.operations/1-draft", "requestId":"abandoned-wait",
            "profileId":"presentations-author-model-v01-draft", "outputMode":"job",
            "action":{"kind":"create", "document":fixture["document"]}
        }}))
        .unwrap();
    let caller = {
        let bridge = bridge.clone();
        let request = request.clone();
        tokio::spawn(async move { bridge.dispatch(request).await })
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while bridge.computation.available_permits() != 0 {
        assert!(Instant::now() < deadline);
        tokio::task::yield_now().await;
    }
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert_eq!(bridge.computation.available_permits(), 0);
    assert_eq!(
        bridge.dispatch(request.clone()).await.unwrap_err().code,
        FailureCode::LimitExceeded
    );
    sql.execute_batch("ROLLBACK").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let stored: Vec<String> = sql
            .prepare("SELECT info FROM jobs")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        if let Some(info) = stored.first() {
            assert_eq!(stored.len(), 1);
            let info: JobInfo = serde_json::from_str(info).unwrap();
            if info.state.terminal() {
                assert_eq!(info.state, JobState::Succeeded);
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "abandoned accepted work never completed"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // Retry resolves the existing receipt, never a second mutation.
    while bridge.computation.available_permits() == 0 {
        tokio::task::yield_now().await;
    }
    assert!(matches!(
        bridge.dispatch(request).await.unwrap(),
        HostResponse::Succeeded { .. }
    ));
    assert_eq!(
        sql.query_row("SELECT count(*) FROM jobs", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(sql);
    Arc::try_unwrap(bridge).ok().unwrap().shutdown().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
