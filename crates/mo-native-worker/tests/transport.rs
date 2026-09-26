#![cfg(unix)]
use mo_native_worker::{CHUNK_BYTES, exchange, exchange_stream};
use std::{
    cell::Cell,
    io::Read,
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};
fn command(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}
#[test]
fn non_send_provider_streams_exact_bytes_with_bounded_read_ahead() {
    let produced = Rc::new(Cell::new(0));
    let n = produced.clone();
    let output = exchange_stream(
        command("exec cat"),
        Duration::from_secs(5),
        &|| false,
        move || {
            let index = n.get();
            if index == 3 {
                return Ok(None);
            }
            n.set(index + 1);
            Ok(Some(vec![index as u8; 4096]))
        },
        |stdout| {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            Ok(bytes)
        },
    )
    .unwrap();
    assert_eq!(produced.get(), 3);
    assert_eq!(
        output,
        [vec![0; 4096], vec![1; 4096], vec![2; 4096]].concat()
    );
}
#[test]
fn deadline_and_cancellation_interrupt_blocked_input_and_wait_for_exit() {
    for cancel in [false, true] {
        let start = Instant::now();
        let produced = Cell::new(0);
        let result = exchange_stream(
            command("exec sleep 30"),
            Duration::from_millis(if cancel { 5000 } else { 100 }),
            &|| cancel && start.elapsed() > Duration::from_millis(80),
            || {
                produced.set(produced.get() + 1);
                Ok(Some(vec![1; CHUNK_BYTES]))
            },
            |_| Ok(()),
        );
        assert!(
            result
                .unwrap_err()
                .contains(if cancel { "cancelled" } else { "timed out" })
        );
        assert!(start.elapsed() < Duration::from_secs(3));
        assert!(
            produced.get() < 12,
            "input was eagerly consumed: {}",
            produced.get()
        );
    }
}
#[test]
fn closed_output_does_not_bypass_process_exit_deadline() {
    let result = exchange(
        command("exec 1>&-; exec sleep 30"),
        vec![],
        Duration::from_millis(100),
        |_| Ok(()),
    );
    assert!(result.unwrap_err().contains("timed out"));
}
#[test]
fn trailing_output_bad_exit_and_provider_failure_are_not_success() {
    assert!(
        exchange(
            command("printf xy"),
            vec![],
            Duration::from_secs(3),
            |out| {
                let mut b = [0; 1];
                out.read_exact(&mut b).map_err(|e| e.to_string())?;
                Ok(())
            }
        )
        .is_err()
    );
    assert!(
        exchange(
            command("exit 3"),
            vec![],
            Duration::from_secs(3),
            |_| Ok(())
        )
        .is_err()
    );
    assert!(
        exchange_stream(
            command("exec cat"),
            Duration::from_secs(3),
            &|| false,
            || Err("source failed".into()),
            |_| Ok(())
        )
        .unwrap_err()
        .contains("source failed")
    );
}
