use super::*;
use std::{
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let nonce = super::super::NEXT_SPOOL.fetch_add(1, Ordering::Relaxed);
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mo-lease-test-{}-{epoch}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("operator-file"), b"keep").unwrap();
        Self(path)
    }
    fn open(&self) -> ExecutionSpoolRoot {
        ExecutionSpoolRoot::open(&self.0).unwrap()
    }
    fn clean(&self) {
        let entries: Vec<_> = fs::read_dir(self.0.join(NAMESPACE))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, [REGISTRY]);
        assert_eq!(fs::read(self.0.join("operator-file")).unwrap(), b"keep");
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

pub(super) fn pause(phase: &str, path: &Path) {
    if std::env::var("MO_SPOOL_TEST_PAUSE").is_ok_and(|value| value == phase) {
        let ack = PathBuf::from(std::env::var_os("MO_SPOOL_TEST_ACK").unwrap());
        fs::write(ack, path.to_str().unwrap()).unwrap();
        loop {
            thread::park_timeout(Duration::from_secs(1));
        }
    }
}

struct Process(Child);
impl Process {
    fn start(root: &Root, mode: &str, phase: &str, path: Option<&Path>, ack: &Path) -> Self {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "execution::tests::process_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("MO_SPOOL_TEST_ROOT", &root.0)
            .env("MO_SPOOL_TEST_MODE", mode)
            .env("MO_SPOOL_TEST_PAUSE", phase)
            .env("MO_SPOOL_TEST_ACK", ack)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        if let Some(path) = path {
            command.env("MO_SPOOL_TEST_PATH", path);
        }
        Self(command.spawn().unwrap())
    }
    fn ack(&mut self, ack: &Path) -> PathBuf {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(value) = fs::read_to_string(ack)
                && !value.is_empty()
            {
                return PathBuf::from(value);
            }
            assert!(
                self.0.try_wait().unwrap().is_none(),
                "helper exited before acknowledgment"
            );
            assert!(Instant::now() < deadline, "helper acknowledgment timeout");
            thread::sleep(Duration::from_millis(5));
        }
    }
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
#[ignore = "subprocess entry; invoked by crash-recovery tests"]
fn process_helper() {
    let root = ExecutionSpoolRoot::open(&PathBuf::from(
        std::env::var_os("MO_SPOOL_TEST_ROOT").unwrap(),
    ))
    .unwrap();
    match std::env::var("MO_SPOOL_TEST_MODE").unwrap().as_str() {
        "create" => {
            let execution = root.create().unwrap();
            fs::write(execution.path().join("pending-output"), b"private").unwrap();
            pause("owner", execution.path());
        }
        "join" => {
            let path = PathBuf::from(std::env::var_os("MO_SPOOL_TEST_PATH").unwrap());
            let _lease = ExecutionSpoolLease::join(&path).unwrap();
            pause("worker", &path);
        }
        "recover" => {
            root.recover(64).unwrap();
        }
        _ => panic!("unknown helper mode"),
    }
}

#[test]
fn active_acquisition_wait_is_bounded_and_cancelled_without_allocating() {
    let root = Root::new();
    let spool = root.open();
    let registry = spool.registry().unwrap();
    let timeout = spool
        .create_with_wait(Duration::from_millis(20), &|| false)
        .err()
        .unwrap();
    assert_eq!(timeout.kind(), io::ErrorKind::TimedOut);
    let cancelled = spool
        .create_with_wait(Duration::from_secs(1), &|| true)
        .err()
        .unwrap();
    assert_eq!(cancelled.kind(), io::ErrorKind::Interrupted);
    drop(registry);
    root.clean();
    let a = spool.clone();
    let b = spool.clone();
    let start = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = start.clone();
    let left = thread::spawn(move || {
        start.wait();
        a.create_with_wait(Duration::from_secs(2), &|| false)
            .unwrap()
    });
    let right = thread::spawn(move || {
        other.wait();
        b.create_with_wait(Duration::from_secs(2), &|| false)
            .unwrap()
    });
    let left = left.join().unwrap();
    let right = right.join().unwrap();
    assert_ne!(left.path(), right.path());
    assert_eq!(spool.recover(64).unwrap().live, 2);
    left.discard_with_wait(Duration::from_secs(1)).unwrap();
    right.discard_with_wait(Duration::from_secs(1)).unwrap();
    root.clean();
}

#[test]
fn timed_out_cleanup_keeps_an_orphan_recoverable_after_the_lock_is_released() {
    let root = Root::new();
    let spool = root.open();
    let owner = spool.create().unwrap();
    let path = owner.path().to_path_buf();
    let registry = spool.registry().unwrap();
    assert_eq!(
        owner
            .discard_with_wait(Duration::from_millis(20))
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(path.exists());
    drop(registry);
    assert_eq!(spool.recover(64).unwrap().reclaimed, 1);
    root.clean();
}

#[test]
#[cfg(unix)]
fn registry_scope_releases_lock_even_while_a_duplicate_descriptor_exists() {
    let root = Root::new();
    let spool = root.open();
    let registry = spool.registry().unwrap();
    // A duplicate shares the open-file description, just like the transient
    // inherited descriptor between fork and exec in another test/process.
    let duplicate = registry.duplicate_descriptor().unwrap();
    assert_eq!(
        spool.registry().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(registry);
    let next = spool
        .registry()
        .expect("registry scope must release its lock");
    drop(next);
    drop(duplicate);
    root.clean();
}

#[test]
#[cfg(unix)]
fn participant_release_ignores_duplicates_but_preserves_independent_workers() {
    let root = Root::new();
    let spool = root.open();
    let owner = spool.create().unwrap();
    let path = owner.path().to_path_buf();
    let owner_duplicate = owner
        .lease
        .as_ref()
        .unwrap()
        .duplicate_descriptor()
        .unwrap();
    let worker = ExecutionSpoolLease::join(&path).unwrap();
    let worker_duplicate = worker._lease.duplicate_descriptor().unwrap();
    assert_eq!(
        owner.discard().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(spool.recover(64).unwrap().live, 1);
    drop(worker);
    assert_eq!(spool.recover(64).unwrap().reclaimed, 1);
    drop(owner_duplicate);
    drop(worker_duplicate);
    root.clean();
}

#[test]
fn owners_and_workers_exclude_recovery_until_last_participant_exits() {
    let root = Root::new();
    let spool = root.open();
    let owner = spool.create().unwrap();
    let path = owner.path().to_path_buf();
    fs::write(path.join("result"), b"retained").unwrap();
    let worker = ExecutionSpoolLease::join(&path).unwrap();
    assert_eq!(spool.recover(64).unwrap().live, 1);
    assert_eq!(
        owner.discard().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(spool.recover(64).unwrap().live, 1);
    assert_eq!(fs::read(path.join("result")).unwrap(), b"retained");
    drop(worker);
    assert_eq!(spool.recover(64).unwrap().reclaimed, 1);
    assert!(ExecutionSpoolLease::join(&path).is_err());
    root.clean();
}

#[test]
fn process_death_at_creation_and_unlink_boundaries_recovers_on_reopen() {
    for phase in ["directory", "leased", "owner"] {
        let root = Root::new();
        let ack = root.0.join("ack");
        let mut process = Process::start(&root, "create", phase, None, &ack);
        let path = process.ack(&ack);
        assert!(path.is_dir());
        let spool = root.open();
        if phase == "owner" {
            assert_eq!(spool.recover(64).unwrap().live, 1);
        } else {
            assert_eq!(
                spool.recover(64).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
        }
        process.kill();
        let recover_ack = root.0.join("recovery-ack");
        let mut recovery = Process::start(&root, "recover", "reclaimed", None, &recover_ack);
        assert_eq!(recovery.ack(&recover_ack), path);
        assert!(!path.exists());
        recovery.kill();
        assert_eq!(root.open().recover(64).unwrap().reclaimed, 0);
        root.clean();
    }
}

#[test]
fn parent_death_cannot_reclaim_a_live_worker() {
    let root = Root::new();
    let owner_ack = root.0.join("owner-ack");
    let mut owner = Process::start(&root, "create", "owner", None, &owner_ack);
    let path = owner.ack(&owner_ack);
    let worker_ack = root.0.join("worker-ack");
    let mut worker = Process::start(&root, "join", "worker", Some(&path), &worker_ack);
    assert_eq!(worker.ack(&worker_ack), path);
    owner.kill();
    let spool = root.open();
    let report = spool.recover(64).unwrap();
    assert_eq!((report.live, report.reclaimed), (1, 0));
    assert_eq!(fs::read(path.join("pending-output")).unwrap(), b"private");
    worker.kill();
    assert_eq!(spool.recover(64).unwrap().reclaimed, 1);
    root.clean();
}

#[test]
fn bounded_recovery_scans_past_live_entries_and_preserves_unknown_siblings() {
    let root = Root::new();
    let spool = root.open();
    let owner = spool.create().unwrap();
    for n in 0..70 {
        // Independent disk fixtures represent interrupted directory creation.
        let path = spool.path.join(format!("execution-999999999-1-{n}"));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("output"), b"orphan").unwrap();
    }
    fs::create_dir(spool.path.join("unrecognized")).unwrap();
    fs::write(spool.path.join("unrecognized/keep"), b"keep").unwrap();
    fs::write(spool.path.join("execution-1-1-01"), b"noncanonical").unwrap();
    let first = spool.recover(64).unwrap();
    assert_eq!(
        (first.live, first.reclaimed, first.unrecognized),
        (1, 64, 2)
    );
    assert!(first.more_reclaimable);
    let second = spool.recover(64).unwrap();
    assert_eq!(
        (second.live, second.reclaimed, second.unrecognized),
        (1, 6, 2)
    );
    assert!(!second.more_reclaimable);
    assert_eq!(
        fs::read(spool.path.join("unrecognized/keep")).unwrap(),
        b"keep"
    );
    owner.discard().unwrap();
}

#[test]
fn missing_registry_is_never_recreated_beside_executions() {
    let root = Root::new();
    let spool = root.open();
    let owner = spool.create().unwrap();
    fs::remove_file(spool.path.join(REGISTRY)).unwrap();
    assert!(ExecutionSpoolRoot::open(&root.0).is_err());
    assert!(!spool.path.join(REGISTRY).exists());
    drop(owner);
}

#[test]
fn concurrent_initializers_and_allocators_keep_live_readers() {
    let root = Root::new();
    let begin = std::sync::Barrier::new(8);
    let owners: Vec<_> = thread::scope(|scope| {
        let mut handles = Vec::new();
        for n in 0..8 {
            let (root, begin) = (&root, &begin);
            handles.push(scope.spawn(move || {
                begin.wait();
                let spool = root.open();
                let deadline = Instant::now() + Duration::from_secs(10);
                let owner = loop {
                    match spool.create() {
                        Ok(owner) => break owner,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline);
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(error) => panic!("allocation failed: {error}"),
                    }
                };
                fs::write(owner.path().join("value"), n.to_string()).unwrap();
                assert_eq!(
                    fs::read_to_string(owner.path().join("value")).unwrap(),
                    n.to_string()
                );
                owner
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    });
    for (n, owner) in owners.iter().enumerate() {
        assert_eq!(
            fs::read_to_string(owner.path().join("value")).unwrap(),
            n.to_string()
        );
    }
    drop(owners);
    root.open().recover(64).unwrap();
    root.clean();
}

#[test]
fn completed_cleanup_relinquishes_path_before_drop() {
    let root = Root::new();
    let spool = root.open();
    let mut owner = spool.create().unwrap();
    let path = owner.path().to_path_buf();
    owner.cleanup().unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("new-owner"), b"keep").unwrap();
    drop(owner);
    assert_eq!(fs::read(path.join("new-owner")).unwrap(), b"keep");
}

#[test]
fn registry_contention_and_capacity_are_explicit_and_retryable() {
    let root = Root::new();
    let spool = root.open();
    let guard = spool.registry().unwrap();
    assert_eq!(
        spool.create().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(
        spool.recover(1).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(guard);
    spool.create().unwrap().discard().unwrap();
    assert_eq!(
        spool.recover(0).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        spool.recover(65).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    for n in 0..MAX_ENTRIES {
        fs::write(spool.path.join(format!("unknown-{n}")), b"keep").unwrap();
    }
    assert_eq!(
        spool.create().err().unwrap().kind(),
        io::ErrorKind::StorageFull
    );
    fs::write(spool.path.join("overflow"), b"keep").unwrap();
    assert!(spool.recover(1).is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed_and_nested_orphan_links_do_not_delete_targets() {
    use std::os::unix::{
        ffi::OsStrExt,
        fs::{PermissionsExt, symlink},
    };
    let root = Root::new();
    let spool = root.open();
    assert_eq!(
        fs::metadata(&spool.path).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(spool.path.join(REGISTRY))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let outside = root.0.join("operator-file");
    symlink(&root.0, spool.path.join("execution-1-1-1")).unwrap();
    let orphan = spool.path.join("execution-1-1-2");
    fs::create_dir(&orphan).unwrap();
    symlink(&outside, orphan.join("nested")).unwrap();
    let report = spool.recover(64).unwrap();
    assert_eq!((report.unrecognized, report.reclaimed), (1, 1));
    assert_eq!(fs::read(&outside).unwrap(), b"keep");
    assert_eq!(
        fs::read_link(spool.path.join("execution-1-1-1"))
            .unwrap()
            .as_os_str()
            .as_bytes(),
        root.0.as_os_str().as_bytes()
    );
    let invalid = spool.path.join("execution-1-1-3");
    fs::create_dir(&invalid).unwrap();
    symlink(&outside, invalid.join(LEASE)).unwrap();
    assert!(spool.recover(64).is_err());
    assert!(ExecutionSpoolLease::join(&invalid).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"keep");
}
