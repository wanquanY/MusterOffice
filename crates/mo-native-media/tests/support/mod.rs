#![allow(dead_code)]
use mo_media::DecodeLimits;
use mo_native_limits::ProcessLimits;
use mo_native_media::{WorkerBundle, WorkerOptions};
use std::{path::PathBuf, time::Duration};
pub fn bundle() -> WorkerBundle {
    WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").expect("explicit worker bundle")),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").expect("trusted test bundle pin"),
    )
    .unwrap()
}
pub fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 30,
            max_read_bytes: 1 << 32,
            max_frame_bytes: 1 << 26,
            max_streams: 16,
            max_pixels: 1 << 24,
            max_audio_samples: 262144,
            max_channels: 8,
            max_decode_steps: 10000,
        },
        process: ProcessLimits {
            additional_address_bytes: 512 << 20,
            cpu_seconds: 60,
            open_files: 32,
        },
        open_timeout: Duration::from_secs(15),
        call_timeout: Duration::from_secs(15),
    }
}
pub fn wav(samples: i32) -> (Vec<u8>, Vec<u8>) {
    let pcm: Vec<_> = (0..samples)
        .flat_map(|i| ((i * 1637 % 64000 - 32000) as i16).to_le_bytes())
        .collect();
    let mut data = Vec::new();
    data.extend(b"RIFF");
    data.extend((36 + pcm.len() as u32).to_le_bytes());
    data.extend(b"WAVEfmt ");
    data.extend(16u32.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(48000u32.to_le_bytes());
    data.extend(96000u32.to_le_bytes());
    data.extend(2u16.to_le_bytes());
    data.extend(16u16.to_le_bytes());
    data.extend(b"data");
    data.extend((pcm.len() as u32).to_le_bytes());
    data.extend(&pcm);
    (data, pcm)
}
pub fn read(
    bytes: &[u8],
    offset: u64,
    destination: &mut [u8],
    max_read: usize,
) -> std::io::Result<usize> {
    let offset = usize::try_from(offset).unwrap();
    assert!(offset <= bytes.len());
    assert!(destination.len() <= mo_media_wire::MAX_READ_BYTES as usize);
    let n = destination.len().min(max_read).min(bytes.len() - offset);
    destination[..n].copy_from_slice(&bytes[offset..offset + n]);
    Ok(n)
}
#[cfg(unix)]
pub fn assert_reaped(pid: u32) {
    use rustix::{
        io::Errno,
        process::{Pid, WaitOptions, waitpid},
    };
    assert!(
        matches!(
            waitpid(Pid::from_raw(pid as i32), WaitOptions::NOHANG),
            Err(Errno::CHILD)
        ),
        "worker must already be reaped"
    );
}
