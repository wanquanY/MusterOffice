use std::{io::Read, process::Command, time::Duration};

pub(super) enum Mode {
    Text,
    Cascade,
    Paragraph,
    Metrics,
    Outlines,
    Lines,
    Geometry,
    Layout,
    Paths,
}

pub(super) fn shape(
    request: Vec<u8>,
    font: Vec<u8>,
    mode: Mode,
) -> Result<String, Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?.with_file_name(if cfg!(windows) {
        "mo-text-worker.exe"
    } else {
        "mo-text-worker"
    });
    let mut command = Command::new(executable);
    match mode {
        Mode::Text => {}
        Mode::Cascade => {
            command.arg("--cascade");
        }
        Mode::Paragraph => {
            command.arg("--paragraph");
        }
        Mode::Outlines => {
            command.arg("--outlines");
        }
        Mode::Metrics => {
            command.arg("--metrics");
        }
        Mode::Paths => {
            command.arg("--paths");
        }
        Mode::Layout => {
            command.arg("--layout");
        }
        Mode::Geometry => {
            command.arg("--geometry");
        }
        Mode::Lines => {
            command.arg("--lines");
        }
    }
    run(command, request, font, Duration::from_secs(30)).map_err(Into::into)
}

fn run(
    command: Command,
    request: Vec<u8>,
    font: Vec<u8>,
    timeout: Duration,
) -> Result<String, String> {
    let header = [
        u32::try_from(request.len())
            .map_err(|e| e.to_string())?
            .to_le_bytes(),
        u32::try_from(font.len())
            .map_err(|e| e.to_string())?
            .to_le_bytes(),
    ]
    .concat();
    super::worker::exchange(command, vec![header, request, font], timeout, |output| {
        let mut length = [0; 4];
        output.read_exact(&mut length).map_err(|e| e.to_string())?;
        let count = u32::from_le_bytes(length) as usize;
        if count > 64 * 1024 * 1024 {
            return Err("shaping response byte limit".into());
        }
        let mut bytes = vec![0; count];
        output.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| e.to_string())
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn timeout_kills_and_reaps_worker_with_blocked_input() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exec sleep 30"]);
        let before = std::time::Instant::now();
        let result = run(
            command,
            vec![0; 1024 * 1024],
            vec![],
            Duration::from_millis(100),
        );
        assert!(result.unwrap_err().contains("timed out"));
        assert!(before.elapsed() < Duration::from_secs(5));
    }
}
