use super::*;
use mo_native_io::ExecutionSpoolRoot;
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Process(Child);
impl Process {
    fn kill(&mut self) {
        self.0.kill().unwrap();
        assert!(!self.0.wait().unwrap().success());
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "subprocess owner; exercised by real-worker recovery test"]
fn owner_helper() {
    let root = PathBuf::from(std::env::var_os("MO_EXPORT_TEST_ROOT").unwrap());
    let spool = ExecutionSpoolRoot::open(&root).unwrap();
    let owner = spool.create().unwrap();
    fs::write(
        std::env::var_os("MO_EXPORT_TEST_ACK").unwrap(),
        owner.path().to_str().unwrap(),
    )
    .unwrap();
    loop {
        thread::park_timeout(Duration::from_secs(1));
    }
}

#[test]
fn actual_worker_survives_owner_death_without_losing_its_input_directory() {
    let root = Root::new();
    let exporter = exporter(&root);
    let ack = root.path().join("owner-ack");
    let mut owner = Process(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "recovery::owner_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("MO_EXPORT_TEST_ROOT", root.spool())
            .env("MO_EXPORT_TEST_ACK", &ack)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    let path = loop {
        if let Ok(value) = fs::read_to_string(&ack)
            && !value.is_empty()
        {
            break PathBuf::from(value);
        }
        assert!(owner.0.try_wait().unwrap().is_none());
        assert!(
            Instant::now() < deadline,
            "owner did not allocate execution"
        );
        thread::sleep(Duration::from_millis(5));
    };
    let mut worker = Process(
        Command::new(worker_path())
            .arg("--spool-dir")
            .arg(&path)
            .arg("--execution-lease-v1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let (snapshot, request, assets) = input(exporter.renderer_identity());
    let header = serde_json::to_vec(&serde_json::json!({
        "version": "musteroffice.native-export/1-draft",
        "request": request, "snapshot": snapshot,
        "assets": assets.0.iter().map(|(info, _)| info).collect::<Vec<_>>()
    }))
    .unwrap();
    let mut input = worker.0.stdin.take().unwrap();
    input
        .write_all(&(header.len() as u32).to_le_bytes())
        .unwrap();
    input.write_all(&header).unwrap();
    input.flush().unwrap();
    // The actual worker has validated the request and opened a real input
    // spool. Keep its stdin open so it remains alive after its owner dies.
    while fs::read_dir(&path).unwrap().count() < 2 {
        assert!(
            worker.0.try_wait().unwrap().is_none(),
            "worker exited before receiving input"
        );
        assert!(Instant::now() < deadline, "worker did not begin input");
        thread::sleep(Duration::from_millis(5));
    }
    owner.kill();
    let report = exporter.recover_spools(64).unwrap();
    assert_eq!((report.live, report.reclaimed), (1, 0));
    assert!(path.is_dir());
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.join("participants.lock"))
        .unwrap();
    assert!(matches!(
        lease.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    drop(lease);
    // A real truncated-input failure closes the worker lease. Recovery can
    // then remove the abandoned execution, without any fabricated success.
    drop(input);
    loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        assert!(Instant::now() < deadline, "worker did not exit after EOF");
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(exporter.recover_spools(64).unwrap().reclaimed, 1);
    root.clean();
}

#[test]
fn registry_wait_obeys_host_cancellation_and_deadline() {
    for cancel in [false, true] {
        let root = Root::new();
        let executable = worker_path();
        let exporter = mo_native_export::NativeExporter::new(
            executable.clone(),
            mo_native_export::executable_digest(&executable).unwrap(),
            root.spool(),
            Duration::from_millis(150),
        )
        .unwrap();
        let registry = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.spool().join(".mo-executions-v1/registry.lock"))
            .unwrap();
        registry.try_lock().unwrap();
        let (snapshot, request, assets) = input(exporter.renderer_identity());
        let polls = Cell::new(0);
        let started = Instant::now();
        let error = exporter
            .prepare(&request, snapshot, &assets, &|| {
                polls.set(polls.get() + 1);
                // Includes initial and resource authorization checks, then waits.
                cancel && polls.get() >= 10
            })
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            if cancel {
                FailureCode::Cancelled
            } else {
                FailureCode::ExecutionInterrupted
            }
        );
        assert!(polls.get() >= 10);
        assert!(started.elapsed() < Duration::from_secs(5));
        if !cancel {
            assert!(error.message.contains("spool acquisition timed out"));
        }
        drop(registry);
        root.clean();
    }
}

#[test]
fn worker_rejects_legacy_cli_and_unmanaged_directories_before_io() {
    let root = Root::new();
    for lease_argument in [false, true] {
        let mut command = Command::new(worker_path());
        command.arg("--spool-dir").arg(root.spool());
        if lease_argument {
            command.arg("--execution-lease-v1");
        }
        let output = command.stdin(Stdio::null()).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(fs::read_dir(root.spool()).unwrap().next().is_none());
    }
}
