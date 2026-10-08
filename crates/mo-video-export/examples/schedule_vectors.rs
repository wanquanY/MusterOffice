//! Native oracle for the independently built public export WASM component.
use mo_common::{DocumentId, RationalTime, Ticks};
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_export::*;
use mo_video_model::*;
use mo_video_render::Sampling;
use serde_json::{Value, json};

fn document(end: RationalTime, numerator: u32, denominator: u32, rate: u32) -> CompiledDocument {
    let root = SequenceId::new("main").unwrap();
    let mut document = VideoDocument::empty(
        DocumentId::new("export-oracle").unwrap(),
        root.clone(),
        Canvas {
            width: 128,
            height: 96,
        },
        FrameRate {
            numerator,
            denominator,
        },
    );
    document.audio_sample_rate = rate;
    let track = TrackId::new("visual").unwrap();
    let clip = ClipId::new("solid").unwrap();
    document
        .sequences
        .get_mut(&root)
        .unwrap()
        .tracks
        .push(track.clone());
    document.tracks.insert(
        track.clone(),
        Track {
            id: track,
            name: "Visual".into(),
            kind: TrackKind::Graphics,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![clip.clone()],
        },
    );
    document.clips.insert(
        clip.clone(),
        Clip {
            id: clip,
            name: "Matte".into(),
            timeline: TimeRange {
                start: RationalTime::new(0, 1).unwrap(),
                end,
            },
            source: ClipSource::Solid {
                rgba: [240, 100, 10, 200],
            },
            effects: vec![],
            audio: AudioTimePolicy::default(),
            audio_context: None,
        },
    );
    CompiledDocument::new(document, CompileLimits::default(), &|| false).unwrap()
}
fn options(audio: ExportAudio) -> ExportOptions {
    ExportOptions {
        profile: mo_media::EncodeProfile::H264WebCodecsAacMp4,
        video_bit_rate: 2_000_000,
        gop_frames: 30,
        audio,
        background_srgb: [24, 24, 24],
        tone_map: None,
        sampling: Sampling::Bilinear,
    }
}
fn request_options(audio: ExportAudio, block: u32) -> Value {
    json!({ "videoBitRate":2000000, "gopFrames":30, "audio":match audio {
        ExportAudio::Omit => json!({"kind":"omit"}),
        ExportAudio::Mono { bit_rate } => json!({"kind":"mono","bitRate":bit_rate}),
        ExportAudio::Stereo { bit_rate } => json!({"kind":"stereo","bitRate":bit_rate}),
    }, "audioBlockFrames":block,"backgroundSrgb":[24,24,24],"sourcePeakNits":null })
}
fn main() {
    let cases: Vec<_> = [
        (24157, 48000, 30000, 1001, 48000, 1024, ExportAudio::Stereo { bit_rate: 128000 }),
        (3003, 1000, 24000, 1001, 44100, 4093, ExportAudio::Mono { bit_rate: 96000 }),
        (3003, 1000, 30, 1, 48000, 1024, ExportAudio::Omit),
        (1, 25, 25, 1, 48000, 1024, ExportAudio::Mono { bit_rate: 96000 }),
    ].into_iter().map(|(ticks, scale, n, d, rate, block, audio)| {
        let document = document(RationalTime::new(ticks, scale).unwrap(), n, d, rate);
        let plan = ExportPlan::new(&document, options(audio), ExportLimits {
            audio_block_frames: block, ..Default::default()
        }, &|| false).unwrap();
        let mut schedule = plan.schedule();
        let mut steps = vec![];
        loop {
            let work = schedule.next();
            let transport = match work {
                ExportWork::Video(v) => Some(v.codec_interval().unwrap()),
                ExportWork::Audio(a) => Some(a.codec_interval().unwrap()),
                ExportWork::Complete => None,
            };
            steps.push(json!({"work":work,"transport":transport}));
            if work == ExportWork::Complete { break; }
            schedule.accept(work).unwrap();
        }
        let c = plan.configuration();
        json!({"document":document.document(),"documentDigest":document.digest(),
            "options":request_options(audio,block),"steps":steps,
            "expected":{"videoFrames":c.video_frames,"videoTimeScale":c.video_time_scale,
                "videoDuration":Ticks::new(c.video_duration),"audioSamples":Ticks::new(c.audio_samples),
                "audioChannels":c.audio_channels,"audioTail":plan.audio_tail()},
            "progress":{"videoFrames":schedule.progress().video_frames,
                "audioSamples":Ticks::new(schedule.progress().audio_samples)}})
    }).collect();
    let tail = document(
        RationalTime::new(10000001, 300000000).unwrap(),
        30,
        1,
        48000,
    );
    println!(
        "{}",
        json!({"format":"musteroffice.export-schedule-vectors/1","cases":cases,
        "unrepresentableTail":{"document":tail.document(),"documentDigest":tail.digest(),
            "options":request_options(ExportAudio::Omit,1024)}})
    );
}
