use crate::{ExportPlan, ExportProgress, ExportSchedule, ExportWork, PlanError, ScheduleError};
use mo_common::Digest;
use mo_media::{EncodeConfig, EncodeStats, EncodeVideoFrame, VideoEncoder};
use mo_time::ExactRatio;
use mo_video_audio::{AudioError, AudioMixer, AudioResources};
use mo_video_render::{RenderError, VisualResources, render_frame};

#[derive(Debug, thiserror::Error)]
pub enum ExportError<E: std::error::Error> {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Schedule(#[from] ScheduleError),
    #[error(transparent)]
    Compile(#[from] mo_video_compile::CompileError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("candidate encoding failed: {0}")]
    Encode(E),
    #[error("export cancelled")]
    Cancelled,
    #[error("export is closed or failed")]
    State,
    #[error("encoder does not match the immutable export plan")]
    Binding,
    #[error("encoder completion counts do not match the plan")]
    Completion,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExportStats {
    pub gamut_clipped_pixels: u64,
    pub tone_mapped_pixels: u64,
    pub chroma_clipped_samples: u64,
    pub peak_video_output_bytes: usize,
    pub peak_composition_bytes: usize,
    pub peak_mix_pcm_bytes: usize,
}
/// Unpublished bytes only. Host must bind component/profile builds, verify the
/// immutable resource closure and encoded output, and atomically publish it.
#[derive(Debug)]
pub struct ExportCandidate {
    pub document_digest: Digest,
    pub configuration: EncodeConfig,
    pub audio_tail: ExactRatio,
    pub encoded: EncodeStats,
    pub render: ExportStats,
}
#[derive(Debug)]
pub enum ExportStep {
    Advanced(ExportProgress),
    /// No input was submitted to the encoder for this step. Prepare the named
    /// source/derived window, then call step again; do not busy-poll missing data.
    WaitingForAudio(AudioError),
    ReadyToVerify(Box<ExportCandidate>),
}
pub struct ExportSession<'a, E: VideoEncoder> {
    plan: ExportPlan<'a>,
    encoder: Option<E>,
    mixer: Option<AudioMixer<'a>>,
    schedule: ExportSchedule,
    stats: ExportStats,
}
impl<'a, E: VideoEncoder> ExportSession<'a, E> {
    pub fn new(plan: ExportPlan<'a>, encoder: E) -> Result<Self, ExportError<E::Error>> {
        if encoder.configuration() != plan.configuration() {
            return Err(ExportError::Binding);
        }
        let mixer = plan
            .options
            .audio
            .layout()
            .map(|layout| AudioMixer::new(plan.document, layout, plan.limits.mix))
            .transpose()?;
        let schedule = plan.schedule();
        Ok(Self {
            plan,
            encoder: Some(encoder),
            mixer,
            schedule,
            stats: ExportStats::default(),
        })
    }
    pub fn progress(&self) -> ExportProgress {
        self.schedule.progress()
    }
    pub fn is_closed(&self) -> bool {
        self.encoder.is_none()
    }
    pub fn abort(&mut self) {
        self.encoder.take();
    }
    pub fn step(
        &mut self,
        visual: &mut dyn VisualResources,
        audio: &mut dyn AudioResources,
        check: &dyn Fn() -> bool,
    ) -> Result<ExportStep, ExportError<E::Error>> {
        if self.encoder.is_none() {
            return Err(ExportError::State);
        }
        let result = self.advance(visual, audio, check);
        if result.is_err() {
            self.abort();
        }
        result
    }
    fn advance(
        &mut self,
        visual: &mut dyn VisualResources,
        audio: &mut dyn AudioResources,
        check: &dyn Fn() -> bool,
    ) -> Result<ExportStep, ExportError<E::Error>> {
        if check() {
            return Err(ExportError::Cancelled);
        }
        let c = self.plan.config;
        let work = self.schedule.next();
        if work == ExportWork::Complete {
            let mut encoder = self.encoder.take().ok_or(ExportError::State)?;
            let encoded = encoder.finish().map_err(ExportError::Encode)?;
            if check() {
                return Err(ExportError::Cancelled);
            }
            if encoded.video_frames != u64::from(c.video_frames)
                || encoded.video_packets != u64::from(c.video_frames)
                || encoded.audio_samples != c.audio_samples as u64
                || encoded.byte_length == 0
            {
                return Err(ExportError::Completion);
            }
            return Ok(ExportStep::ReadyToVerify(Box::new(ExportCandidate {
                document_digest: self.plan.document_digest().clone(),
                configuration: c,
                audio_tail: self.plan.audio_tail().clone(),
                encoded,
                render: self.stats,
            })));
        }
        if let ExportWork::Video(sample) = work {
            let frame =
                self.plan
                    .document
                    .frame(i64::from(sample.index), self.plan.limits.query, check)?;
            let rendered = render_frame(
                &frame.value,
                visual,
                self.plan.options.sampling,
                self.plan.limits.render,
                check,
            )?;
            let image = rendered.image.to_i420(self.plan.output, check)?;
            if check() {
                return Err(ExportError::Cancelled);
            }
            self.encoder
                .as_mut()
                .ok_or(ExportError::State)?
                .video(EncodeVideoFrame {
                    pts: sample.pts.get(),
                    duration: sample.duration.get(),
                    planes: image.planes(),
                    strides: image.strides(),
                })
                .map_err(ExportError::Encode)?;
            self.stats.gamut_clipped_pixels += image.colors.gamut_clipped_pixels as u64;
            self.stats.tone_mapped_pixels += image.colors.tone_mapped_pixels as u64;
            self.stats.chroma_clipped_samples += image.chroma_clipped_samples as u64;
            self.stats.peak_composition_bytes = self
                .stats
                .peak_composition_bytes
                .max(rendered.stats.peak_temporary_bytes);
            self.stats.peak_video_output_bytes = self.stats.peak_video_output_bytes.max(
                rendered.image.pixels().len() * 16 + image.byte_length() + image.scratch_bytes,
            );
        } else if let ExportWork::Audio(sample) = work {
            let a = sample.first_sample.get();
            let frames = sample.frames;
            let mixer = self.mixer.as_mut().ok_or(ExportError::Binding)?;
            let mixed = match mixer.render(audio, a, frames, check) {
                Ok(value) => value,
                Err(
                    error @ (AudioError::PendingSource { .. } | AudioError::PendingDerived { .. }),
                ) => {
                    if check() {
                        return Err(ExportError::Cancelled);
                    }
                    return Ok(ExportStep::WaitingForAudio(error));
                }
                Err(error) => return Err(error.into()),
            };
            if check() {
                return Err(ExportError::Cancelled);
            }
            self.encoder
                .as_mut()
                .ok_or(ExportError::State)?
                .audio(a, mixed.pcm.samples())
                .map_err(ExportError::Encode)?;
            self.stats.peak_mix_pcm_bytes = self
                .stats
                .peak_mix_pcm_bytes
                .max(mixed.stats.peak_pcm_bytes);
        }
        if check() {
            return Err(ExportError::Cancelled);
        }
        Ok(ExportStep::Advanced(self.schedule.accept(work)?))
    }
}
