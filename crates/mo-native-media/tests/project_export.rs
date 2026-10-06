mod encode_support;
#[path = "../../mo-video-export/tests/support/mod.rs"]
mod project;
mod support;
use encode_support::{Sink, decoded};
use mo_media::VideoEncoder;
use mo_media::*;
use mo_native_media::{NativeEncoder, WorkerError};
use mo_video_export::*;
use serde_json::json;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Inputs {
    video: Vec<u8>,
    audio: Vec<f32>,
}
struct RecordingEncoder<'a> {
    inner: NativeEncoder<'a>,
    config: EncodeConfig,
    inputs: Rc<RefCell<Inputs>>,
}
impl VideoEncoder for RecordingEncoder<'_> {
    type Error = WorkerError;
    fn configuration(&self) -> EncodeConfig {
        self.config
    }
    fn video(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), WorkerError> {
        let planes = frame.planes;
        self.inner.video(frame)?;
        for plane in planes {
            self.inputs.borrow_mut().video.extend_from_slice(plane);
        }
        Ok(())
    }
    fn audio(&mut self, first: i64, samples: &[f32]) -> Result<(), WorkerError> {
        self.inner.audio(first, samples)?;
        self.inputs.borrow_mut().audio.extend_from_slice(samples);
        Ok(())
    }
    fn finish(&mut self) -> Result<EncodeStats, WorkerError> {
        self.inner.finish()
    }
}
fn export(
    profile: EncodeProfile,
    rate: u32,
    chunk: u32,
    audio: bool,
) -> (ExportCandidate, Vec<u8>, Inputs) {
    let mut d = project::document();
    d.audio_sample_rate = rate;
    let compiled = project::compile(d);
    let mut options = project::options();
    options.profile = profile;
    if !audio {
        options.audio = ExportAudio::Omit;
    }
    let plan = ExportPlan::new(
        &compiled,
        options,
        ExportLimits {
            audio_block_frames: chunk,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    let config = plan.configuration();
    let sink = Sink::default();
    let inputs = Rc::new(RefCell::new(Inputs::default()));
    let inner = NativeEncoder::open(
        &support::bundle(),
        Sink(sink.0.clone(), 113),
        config,
        encode_support::options(),
        || false,
    )
    .unwrap();
    let pid = inner.process_id().unwrap();
    let mut session = ExportSession::new(
        plan,
        RecordingEncoder {
            inner,
            config,
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let mut audio = project::Audio::default();
    let candidate = loop {
        match session
            .step(&mut project::Visual, &mut audio, &|| false)
            .unwrap()
        {
            ExportStep::Advanced(_) => (),
            ExportStep::WaitingForAudio(_) => panic!("fixture resources are ready"),
            ExportStep::ReadyToVerify(value) => break *value,
        }
    };
    drop(session);
    support::assert_reaped(pid);
    let input = Rc::try_unwrap(inputs).ok().unwrap().into_inner();
    let bytes = Rc::try_unwrap(sink.0).ok().unwrap().into_inner();
    assert_eq!(candidate.encoded.byte_length, bytes.len() as u64);
    (candidate, bytes, input)
}
#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn project_composition_and_mix_reach_lossless_container_without_clock_or_pixel_changes() {
    for (rate, audio) in [(48000, true), (44100, true), (48000, false)] {
        let (candidate, bytes, input) = export(EncodeProfile::Ffv1PcmMp4, rate, 997, audio);
        let (video, samples) = decoded(&bytes, candidate.configuration);
        assert_eq!(video, input.video);
        assert_eq!(samples, input.audio);
        assert_eq!(candidate.encoded.video_frames, 16);
        let (other, other_bytes, other_input) =
            export(EncodeProfile::Ffv1PcmMp4, rate, 4093, audio);
        assert_eq!(input.video, other_input.video);
        assert_eq!(input.audio, other_input.audio);
        assert_eq!(decoded(&other_bytes, other.configuration), (video, samples));
    }
}
#[test]
#[ignore = "explicit evidence output; optional hardware profile is required when requested"]
fn actual_project_export_files() {
    let output = std::path::PathBuf::from(
        std::env::var_os("MO_PROJECT_EXPORT_OUTPUT").expect("evidence output"),
    );
    std::fs::create_dir(&output).unwrap();
    let mut profiles = vec![("lossless", EncodeProfile::Ffv1PcmMp4)];
    if std::env::var_os("MO_PROJECT_EXPORT_HARDWARE").is_some() {
        profiles.push(("hardware", EncodeProfile::H264VideoToolboxAacMp4));
    }
    let mut rows = vec![];
    for (label, profile) in profiles {
        for (rate, audio) in [(48000, true), (44100, true), (48000, false)] {
            let name = format!("{label}-{rate}-{}", if audio { "audio" } else { "silent" });
            let (candidate, bytes, input) = export(profile, rate, 997, audio);
            let c = candidate.configuration;
            let (video, samples) = decoded(&bytes, c);
            std::fs::write(output.join(format!("{name}.mp4")), bytes).unwrap();
            std::fs::write(output.join(format!("{name}.input.yuv")), input.video).unwrap();
            std::fs::write(output.join(format!("{name}.decoded.yuv")), video).unwrap();
            for (suffix, pcm) in [("input", input.audio), ("decoded", samples)] {
                let bytes: Vec<_> = pcm.into_iter().flat_map(f32::to_le_bytes).collect();
                std::fs::write(output.join(format!("{name}.{suffix}.f32")), bytes).unwrap();
            }
            rows.push(json!({"name":name,"documentDigest":candidate.document_digest,"audioTail":candidate.audio_tail,
                "config":{"width":c.width,"height":c.height,"videoTimeScale":c.video_time_scale,"videoDuration":c.video_duration,
                    "videoFrames":c.video_frames,"frameRateNum":c.frame_rate_num,"frameRateDen":c.frame_rate_den,
                    "audioSamples":c.audio_samples,"audioSampleRate":c.audio_sample_rate,"audioBitRate":c.audio_bit_rate},
                "peakVideoOutputBytes":candidate.render.peak_video_output_bytes,"peakMixPcmBytes":candidate.render.peak_mix_pcm_bytes}));
        }
    }
    std::fs::write(
        output.join("exports.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}
