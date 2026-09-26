//! Shared bounded isolated-worker transport.
pub(super) use mo_native_worker::exchange;
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };
    #[test]
    fn closed_stdout_does_not_bypass_process_deadline() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exec 1>&-; exec sleep 30"]);
        let before = Instant::now();
        let result = exchange(command, vec![], Duration::from_millis(100), |_| Ok(()));
        assert!(result.unwrap_err().contains("timed out"));
        assert!(before.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn nonzero_exit_rejects_even_complete_response() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exit 3"]);
        assert!(
            exchange(command, vec![], Duration::from_secs(2), |_| Ok(()))
                .unwrap_err()
                .contains("unsuccessfully")
        );
    }
}
