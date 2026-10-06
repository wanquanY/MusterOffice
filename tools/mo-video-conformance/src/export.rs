//! Captures exact encoder inputs in both targets; this is not a fake file encoder
//! exposed to a product. Real native file checks live in mo-media-ffmpeg tests.
#[path = "../../../crates/mo-video-export/tests/support/mod.rs"]
mod project;
use mo_media::{EncodeConfig, EncodeStats, EncodeVideoFrame, VideoEncoder};
use mo_video_export::*;
use mo_video_render::{I420_PROFILE, I420Options, LinearImage, PixelLimits, ToneMap};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    chunk: u32,
    rate: u32,
    audio: bool,
}
#[derive(Default)]
struct Captured {
    video: Vec<Value>,
    audio: Vec<u32>,
}
struct Capture {
    config: EncodeConfig,
    output: Rc<RefCell<Captured>>,
}
impl VideoEncoder for Capture {
    type Error = std::io::Error;
    fn configuration(&self) -> EncodeConfig {
        self.config
    }
    fn video(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), Self::Error> {
        self.output
            .borrow_mut()
            .video
            .push(json!({"pts":frame.pts,"duration":frame.duration,"planes":frame.planes}));
        Ok(())
    }
    fn audio(&mut self, first: i64, samples: &[f32]) -> Result<(), Self::Error> {
        let mut output = self.output.borrow_mut();
        assert_eq!(
            first as usize * self.config.audio_channels as usize,
            output.audio.len()
        );
        output.audio.extend(samples.iter().map(|v| v.to_bits()));
        Ok(())
    }
    fn finish(&mut self) -> Result<EncodeStats, Self::Error> {
        let out = self.output.borrow();
        Ok(EncodeStats {
            byte_length: 1,
            write_bytes: 1,
            video_frames: out.video.len() as u64,
            video_packets: out.video.len() as u64,
            audio_samples: if self.config.audio_channels == 0 {
                0
            } else {
                out.audio.len() as u64 / u64::from(self.config.audio_channels)
            },
            audio_packets: 0,
            peak_pending_bytes: 0,
        })
    }
}
fn spatial() -> Vec<Value> {
    [(2,2,false),(8,10,false),(18,14,true)].into_iter().map(|(w,h,hdr)|{
        let pixels=(0..w*h).map(|i|{
            let alpha=(i%5) as f32/4.;
            let scale=if hdr {4.} else {1.};
            [((i*7)%31) as f32/30.*scale*alpha,((i*3)%19) as f32/18.*scale*alpha,((i*11)%23) as f32/22.*scale*alpha,alpha]
        }).collect();
        let image=LinearImage::from_pixels(w,h,pixels,hdr,PixelLimits::default(),&||false).unwrap();
        let yuv=image.to_i420(I420Options {background_srgb:[17,43,89],tone_map:hdr.then_some(ToneMap::ReinhardLuminanceV1 {source_peak_nits:1000.}),max_bytes:4096,max_temporary_bytes:4096},&||false).unwrap();
        json!({"width":w,"height":h,"planes":yuv.planes(),"gamut":yuv.colors.gamut_clipped_pixels,"tone":yuv.colors.tone_mapped_pixels,
            "clippedChroma":yuv.chroma_clipped_samples,"scratch":yuv.scratch_bytes})
    }).collect()
}
fn run(input: &str) -> Result<Value, String> {
    if input.len() > 1024 {
        return Err("fixture too large".into());
    }
    let fixture: Fixture = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let mut d = project::document();
    d.audio_sample_rate = fixture.rate;
    let document = mo_video_compile::CompiledDocument::new(
        d,
        mo_video_compile::CompileLimits::default(),
        &|| false,
    )
    .map_err(|e| e.to_string())?;
    let mut options = project::options();
    if !fixture.audio {
        options.audio = ExportAudio::Omit;
    }
    let plan = ExportPlan::new(
        &document,
        options,
        ExportLimits {
            audio_block_frames: fixture.chunk,
            ..Default::default()
        },
        &|| false,
    )
    .map_err(|e| e.to_string())?;
    let config = plan.configuration();
    let output = Rc::new(RefCell::new(Captured::default()));
    let mut session = ExportSession::new(
        plan,
        Capture {
            config,
            output: output.clone(),
        },
    )
    .map_err(|e| e.to_string())?;
    let mut audio = project::Audio::default();
    let candidate = loop {
        match session
            .step(&mut project::Visual, &mut audio, &|| false)
            .map_err(|e| e.to_string())?
        {
            ExportStep::Advanced(_) => (),
            ExportStep::WaitingForAudio(_) => {
                return Err("fixture audio unexpectedly pending".into());
            }
            ExportStep::ReadyToVerify(value) => break value,
        }
    };
    let out = output.borrow();
    Ok(
        json!({"status":"complete","profile":PROFILE,"i420Profile":I420_PROFILE,"documentDigest":candidate.document_digest,
        "videoClock":config.video_time_scale,"audioClock":config.audio_sample_rate,"audioSamples":config.audio_samples,
        "video":out.video,"pcmBits":out.audio,"spatial":spatial()}),
    )
}
#[wasm_bindgen]
pub fn video_export_conformance(input: &str) -> String {
    match run(input) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"status":"invalidFixture","error":error}).to_string(),
    }
}
