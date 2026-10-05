#![cfg(any(target_os = "macos", target_os = "linux"))]
use mo_native_limits::{ProcessLimits, apply_to_current_worker};
use std::{
    io::{Read, Write},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

fn profile() -> ProcessLimits {
    ProcessLimits {
        additional_address_bytes: 32 << 20,
        cpu_seconds: 10,
        open_files: 32,
    }
}
/// Never install limits in the parent test runner or the user's application.
fn child(case: &str) -> Output {
    let mut process = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "isolated_child", "--nocapture"])
        .env("MO_LIMIT_TEST_CASE", case)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while process.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(15) {
            process.kill().unwrap();
            process.wait().unwrap();
            panic!("resource-limit child did not terminate");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = process.wait_with_output().unwrap();
    println!(
        "process-limit-evidence: {}",
        serde_json::json!({
            "case": case, "elapsedMillis": start.elapsed().as_millis(),
            "exit": output.status.to_string(), "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr)
        })
    );
    output
}
#[test]
fn virtual_growth_is_bounded_and_small_allocations_still_work() {
    let result = child("memory");
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("allocation-rejected"));
}
#[test]
fn cpu_loop_is_terminated_by_kernel() {
    use std::os::unix::process::ExitStatusExt;
    for case in ["cpu", "cpu-inherited-signal"] {
        let result = child(case);
        assert!(String::from_utf8_lossy(&result.stdout).contains("limits-installed"));
        // Linux may escalate directly to SIGKILL at the hard ceiling. Darwin emits
        // SIGXCPU; both must be an OS signal, never a voluntary successful return.
        assert!(
            result.status.signal().is_some_and(|signal| [
                rustix::process::Signal::KILL.as_raw(),
                rustix::process::Signal::XCPU.as_raw()
            ]
            .contains(&signal)),
            "{:?}",
            result.status
        );
    }
}
#[test]
fn inherited_stricter_limits_are_preserved() {
    let result = child("inherited");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn excess_file_descriptors_are_rejected_by_kernel() {
    let result = child("descriptors");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("descriptor-rejected"));
}
#[test]
fn invalid_admission_does_not_change_process_limits() {
    let result = child("invalid");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn regular_file_writes_are_disabled_but_protocol_pipes_remain_writable() {
    use std::os::unix::process::ExitStatusExt;
    let result = child("file");
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("pipe-remains-writable"));
    assert!(
        result.status.signal() == Some(rustix::process::Signal::XFSZ.as_raw())
            || (result.status.success() && output.contains("file-growth-refused")),
        "{:?}",
        result.status
    );
}
#[test]
#[ignore = "invoked only by parent tests in a disposable child process"]
fn isolated_child() {
    use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};
    let case = std::env::var("MO_LIMIT_TEST_CASE").expect("child case");
    let mut limits = profile();
    if case == "invalid" {
        let before = getrlimit(Resource::As);
        limits.additional_address_bytes = 0;
        assert!(apply_to_current_worker(limits).is_err());
        assert_eq!(getrlimit(Resource::As), before);
        return;
    }
    if case == "inherited" {
        setrlimit(
            Resource::Cpu,
            Rlimit {
                current: Some(3),
                maximum: Some(4),
            },
        )
        .unwrap();
        setrlimit(
            Resource::Nofile,
            Rlimit {
                current: Some(16),
                maximum: Some(24),
            },
        )
        .unwrap();
    }
    // Open a regular file before the ceiling; setting a zero size limit does not
    // prevent open(), only growth. The parent accepts either SIGXFSZ or an actual
    // failed growth operation. No signal disposition is changed in any process.
    let mut file = if case == "file" {
        let path = std::env::temp_dir().join(format!("mo-limit-test-{}", std::process::id()));
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::fs::remove_file(path).unwrap();
        Some(f)
    } else {
        None
    };
    if case.starts_with("cpu") {
        limits.cpu_seconds = 1;
    }
    if case == "cpu-inherited-signal" {
        inherit_ignored_and_blocked_cpu();
    }
    let applied = apply_to_current_worker(limits).unwrap();
    println!(
        "limits-installed {}",
        serde_json::to_string(&applied).unwrap()
    );
    std::io::stdout().flush().unwrap();
    assert_eq!(getrlimit(Resource::Core).current, Some(0));
    assert_eq!(getrlimit(Resource::Fsize).current, Some(0));
    assert!(
        applied.address_ceiling_bytes
            <= applied.startup_virtual_bytes + limits.additional_address_bytes
    );
    match case.as_str() {
        "memory" => {
            let mut small = Vec::<u8>::new();
            small.try_reserve_exact(65536).unwrap();
            small.resize(65536, 37);
            assert_eq!(small[65000], 37);
            let mut too_big = Vec::<u8>::new();
            assert!(too_big.try_reserve_exact(64 << 20).is_err());
            println!("allocation-rejected");
        }
        "cpu" | "cpu-inherited-signal" => loop {
            std::hint::spin_loop();
        },
        "inherited" => {
            assert_eq!(applied.cpu_seconds, 3);
            assert_eq!(applied.open_files, 16);
        }
        "descriptors" => {
            let mut handles = Vec::new();
            loop {
                match std::fs::File::open("/dev/null") {
                    Ok(file) => {
                        handles.push(file);
                        assert!(handles.len() < applied.open_files as usize);
                    }
                    Err(error) => {
                        assert_eq!(
                            error.raw_os_error(),
                            Some(rustix::io::Errno::MFILE.raw_os_error())
                        );
                        println!("descriptor-rejected");
                        break;
                    }
                }
            }
        }
        "file" => {
            let file = file.as_mut().unwrap();
            file.write_all(&[]).unwrap();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            assert!(data.is_empty());
            println!("pipe-remains-writable");
            std::io::stdout().flush().unwrap();
            assert!(file.write_all(&[1]).is_err());
            assert_eq!(file.metadata().unwrap().len(), 0);
            println!("file-growth-refused");
        }
        _ => panic!("unknown child case"),
    }
}

#[allow(unsafe_code)]
fn inherit_ignored_and_blocked_cpu() {
    // This function runs ONLY in the disposable child. Model an embedding host
    // that ignores and blocks SIGXCPU; exec alone does not reset that policy.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        libc::sigemptyset(&mut action.sa_mask);
        action.sa_sigaction = libc::SIG_IGN;
        assert_eq!(
            libc::sigaction(libc::SIGXCPU, &action, std::ptr::null_mut()),
            0
        );
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGXCPU);
        assert_eq!(
            libc::pthread_sigmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut()),
            0
        );
        let mut actual: libc::sigaction = std::mem::zeroed();
        assert_eq!(
            libc::sigaction(libc::SIGXCPU, std::ptr::null(), &mut actual),
            0
        );
        assert_eq!(actual.sa_sigaction, libc::SIG_IGN);
        assert_eq!(
            libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut mask),
            0
        );
        assert_eq!(libc::sigismember(&mask, libc::SIGXCPU), 1);
    }
}
