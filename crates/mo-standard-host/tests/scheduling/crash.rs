//! Kill a real scheduler process after durable claim, then restart its owner.
use super::*;
use std::process::{Command, Stdio};

#[test]
fn killed_scheduler_recovers_lease_and_executes_remaining_queue() {
    let db = Database::new();
    let mut host = db.host();
    let interrupted = host
        .submit(&context(), request("interrupted"), time(10))
        .unwrap();
    let queued = host
        .submit(&context(), request("survivor"), time(11))
        .unwrap();
    drop(host);
    let ready = db.0.join("claimed");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "crash::scheduler_child",
            "--ignored",
            "--nocapture",
        ])
        .env("MO_SCHEDULER_TEST_DB", db.path())
        .env("MO_SCHEDULER_TEST_READY", &ready)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            panic!("scheduler exited before durable claim");
        }
        thread::sleep(Duration::from_millis(5));
    }
    let observed = ready.exists();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(observed, "scheduler never reached durable claim");
    let mut host = db.host();
    assert_eq!(
        host.get_job(&context(), &interrupted.id, time(50))
            .unwrap()
            .state,
        JobState::Running
    );
    assert_eq!(
        host.get_job(&context(), &queued.id, time(50))
            .unwrap()
            .state,
        JobState::Queued
    );
    let runtime =
        NativeRuntime::start(db.config(1), context(), options(), Arc::new(|| time(151))).unwrap();
    wait(|| {
        host.get_job(&context(), &queued.id, time(151))
            .unwrap()
            .state
            == JobState::Succeeded
    });
    runtime.shutdown().unwrap();
    let result = host
        .get_job(&context(), &interrupted.id, time(151))
        .unwrap();
    let Some(TerminalResult::Failed { error }) = result.result else {
        panic!("interrupted receipt required")
    };
    assert_eq!(error.code, FailureCode::ExecutionInterrupted);
    assert_eq!(result.fence.get(), 1);
    assert_eq!(
        db.sql()
            .query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
#[ignore = "subprocess entry, executed and killed by parent"]
fn scheduler_child() {
    let path = PathBuf::from(std::env::var_os("MO_SCHEDULER_TEST_DB").unwrap());
    let ready = PathBuf::from(std::env::var_os("MO_SCHEDULER_TEST_READY").unwrap());
    let config = StandardHostConfig::new(
        path.clone(),
        Digest::from_sha256([1; 32]),
        HostLimits {
            lease_ms: 100,
            ..HostLimits::default()
        },
    );
    let runtime = NativeRuntime::start(
        config,
        context(),
        RuntimeOptions {
            workers: 1,
            ..options()
        },
        Arc::new(move || {
            let running = Connection::open(&path)
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM jobs WHERE json_extract(info,'$.state')='running'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap();
            if running > 0 {
                std::fs::write(&ready, b"claim committed").unwrap();
                loop {
                    thread::park();
                }
            }
            time(50)
        }),
    )
    .unwrap();
    loop {
        runtime.health().unwrap();
        thread::sleep(Duration::from_millis(10));
    }
}
