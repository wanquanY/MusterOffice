use mo_common::{ByteLength, DocumentId};
use mo_media::{DecodeLimits, ReadAt};
use mo_native_limits::ProcessLimits;
use mo_native_media::WorkerOptions;
use mo_video_compile::{CompileLimits, CompiledDocument, MediaBinding};
use mo_video_model::*;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    time::Duration,
};
fn time(n: i64) -> mo_common::RationalTime {
    mo_common::RationalTime::new(n, 30000).unwrap()
}
pub fn document(binding: &MediaBinding, length: u64, reverse: bool) -> CompiledDocument {
    let MediaStream::Video { available, .. } = binding.descriptor else {
        panic!("video");
    };
    let span = || available;
    let root = SequenceId::new("root").unwrap();
    let track = TrackId::new("v1").unwrap();
    let clip = ClipId::new("clip1").unwrap();
    let mut doc = VideoDocument::empty(
        DocumentId::new("actual-media").unwrap(),
        root.clone(),
        Canvas {
            width: 128,
            height: 96,
        },
        FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
    );
    doc.resources.insert(
        binding.resource.clone(),
        Resource {
            id: binding.resource.clone(),
            digest: binding.digest.clone(),
            byte_length: ByteLength::new(length),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(binding.stream.clone(), binding.descriptor.clone())]),
            },
        },
    );
    doc.sequences
        .get_mut(&root)
        .unwrap()
        .tracks
        .push(track.clone());
    doc.tracks.insert(
        track.clone(),
        Track {
            id: track,
            name: "Video".into(),
            kind: TrackKind::Video,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![clip.clone()],
        },
    );
    let (a, b) = if reverse {
        (span().end, span().start)
    } else {
        (span().start, span().end)
    };
    doc.clips.insert(
        clip.clone(),
        Clip {
            audio_context: None,
            id: clip,
            name: "Generated fixture".into(),
            timeline: span(),
            source: ClipSource::Media {
                resource: binding.resource.clone(),
                stream: binding.stream.clone(),
                time_map: TimeMap {
                    anchors: vec![
                        TimeAnchor {
                            local: time(0),
                            source: a,
                        },
                        TimeAnchor {
                            local: span().end,
                            source: b,
                        },
                    ],
                },
            },
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
    CompiledDocument::new(doc, CompileLimits::default(), &|| false).unwrap()
}
pub fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 30,
            max_read_bytes: 1 << 32,
            max_frame_bytes: 1 << 20,
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
        call_timeout: Duration::from_secs(10),
    }
}

pub fn reader(path: &Path) -> impl ReadAt + 'static {
    let mut file = File::open(path).unwrap();
    move |at, out: &mut [u8]| {
        file.seek(SeekFrom::Start(at))?;
        let n = out.len().min(997);
        file.read(&mut out[..n])
    }
}
pub fn reaped(pid: u32) {
    use rustix::{
        io::Errno,
        process::{Pid, WaitOptions, waitpid},
    };
    assert!(matches!(
        waitpid(Pid::from_raw(pid as i32), WaitOptions::NOHANG),
        Err(Errno::CHILD)
    ));
}
