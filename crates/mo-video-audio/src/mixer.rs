//! Hierarchical block DSP. Every sequence is processed on the project sample
//! lattice before its parent's rate, automation and mix stages are applied.
use crate::{
    AudioError, AudioResources, ChannelLayout, PcmBlock, PcmLimits, ResampleLimits, Result,
    SincResampler, cancel, envelope::Envelope,
};
use mo_time::ExactRatio as R;
use mo_video_compile::{AudioSpanSource, AudioSpanVoice, CompiledDocument, QueryLimits};
use mo_video_model::{AudioRatePolicy, SampleSide, SequenceId};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct MixLimits {
    pub pcm: PcmLimits,
    pub max_active_pcm_bytes: usize,
    pub max_kernel_bytes: usize,
    pub max_retime_plan_bytes: usize,
    pub max_multiply_adds: usize,
    pub max_automation_steps: usize,
    pub max_visits: usize,
    pub max_voices: usize,
    pub max_depth: usize,
    pub arithmetic_bits: u64,
}
impl Default for MixLimits {
    fn default() -> Self {
        Self {
            pcm: PcmLimits::default(),
            max_active_pcm_bytes: 256 << 20,
            max_kernel_bytes: 32 << 20,
            max_retime_plan_bytes: 16 << 20,
            max_multiply_adds: 64_000_000,
            max_automation_steps: 8_000_000,
            max_visits: 1_000_000,
            max_voices: 4096,
            max_depth: 32,
            arithmetic_bits: 256,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixStats {
    pub voices: usize,
    pub visited: usize,
    pub source_frames: usize,
    pub multiply_adds: usize,
    pub automation_steps: usize,
    /// Core-owned active PCM and f64 accumulators, excluding provider/codec/cache
    /// allocations, plans and retained filter kernels (reported separately).
    pub peak_pcm_bytes: usize,
    pub retained_kernel_bytes: usize,
    pub retained_retime_plan_bytes: usize,
}
#[derive(Debug)]
pub struct MixedAudio {
    pub pcm: PcmBlock,
    pub stats: MixStats,
}
/// Bounded reusable filter cache; no sample history and no host resource cache.
/// An error/cancelled render never returns a partial block. Harmless compiled
/// kernels may remain cached. Reconstruct to discard the cache explicitly.
pub struct AudioMixer<'a> {
    document: MixerDocument<'a>,
    output: ChannelLayout,
    limits: MixLimits,
    kernels: Vec<(R, Arc<SincResampler>)>,
    retime_plans: crate::retime_cache::RetimeCache,
}
// Either host-borrowed (native) or explicitly shared (long-lived WASM handle).
// Both variants execute the same mixer and retain the same bounded caches.
enum MixerDocument<'a> {
    Borrowed(&'a CompiledDocument),
    Shared(Arc<CompiledDocument>),
}
impl std::ops::Deref for MixerDocument<'_> {
    type Target = CompiledDocument;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(document) => document,
            Self::Shared(document) => document,
        }
    }
}
impl AudioMixer<'static> {
    /// Own a share of one immutable compiled revision. No leaked allocation or
    /// per-window recompilation; dropping the last owner releases its indexes.
    pub fn from_shared(
        document: Arc<CompiledDocument>,
        output: ChannelLayout,
        limits: MixLimits,
    ) -> Result<Self> {
        Self::construct(MixerDocument::Shared(document), output, limits)
    }
}
impl<'a> AudioMixer<'a> {
    pub fn new(
        document: &'a CompiledDocument,
        output: ChannelLayout,
        limits: MixLimits,
    ) -> Result<Self> {
        Self::construct(MixerDocument::Borrowed(document), output, limits)
    }
    fn construct(
        document: MixerDocument<'a>,
        output: ChannelLayout,
        limits: MixLimits,
    ) -> Result<Self> {
        if limits.max_depth == 0
            || limits.max_depth > 64
            || !(64..=4096).contains(&limits.arithmetic_bits)
            || limits.max_active_pcm_bytes > 1 << 30
            || limits.max_kernel_bytes > 128 << 20
            || limits.max_retime_plan_bytes > 128 << 20
        {
            return Err(AudioError::Invalid("mix limits"));
        }
        Ok(Self {
            document,
            output,
            limits,
            kernels: Vec::new(),
            retime_plans: crate::retime_cache::RetimeCache::default(),
        })
    }
    /// Release reusable DSP caches without recompiling the immutable document.
    pub fn clear(&mut self) {
        self.kernels.clear();
        self.retime_plans = crate::retime_cache::RetimeCache::default();
    }
    pub fn render(
        &mut self,
        resources: &mut dyn AudioResources,
        first: i64,
        frames: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<MixedAudio> {
        cancel(check)?;
        let end = first
            .checked_add(i64::from(frames))
            .ok_or(AudioError::Budget)?;
        if first < 0 || end > self.document.audio_sample_count()? {
            return Err(AudioError::Invalid("audio block outside project"));
        }
        let root = self.document.document().root_sequence.clone();
        let mut session = Session {
            mixer: self,
            resources,
            check,
            stats: MixStats::default(),
            active_bytes: 0,
        };
        let pcm = session.sequence(&root, first, frames, 1)?;
        session.stats.retained_kernel_bytes = session
            .mixer
            .kernels
            .iter()
            .map(|(_, k)| k.kernel_bytes())
            .sum();
        session.stats.retained_retime_plan_bytes = session.mixer.retime_plans.retained_bytes();
        cancel(check)?;
        Ok(MixedAudio {
            pcm,
            stats: session.stats,
        })
    }
    /// Materialize input for a continuous derived stage. Child sequences are
    /// rendered on their own lattice with their own pitch policies first. A
    /// host prepares missing child-derived data before retrying this window.
    pub fn render_retime_input(
        &mut self,
        resources: &mut dyn AudioResources,
        request: &crate::RetimeRequest,
        first: u64,
        frames: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<MixedAudio> {
        request.validate_document(&self.document)?;
        if request.layout() != &self.output
            || &resources.retime_profile()? != request.profile()
            || frames == 0
            || frames > 65536
            || first
                .checked_add(u64::from(frames))
                .is_none_or(|n| n > request.plan().schedule().source_frames())
        {
            return Err(AudioError::Invalid("retime input request binding"));
        }
        let plan = request.plan();
        let offset = R::integer(i64::try_from(first).map_err(|_| AudioError::Budget)?)
            .mul(&R::integer(plan.direction), self.limits.arithmetic_bits)?
            .div(
                &R::integer(i64::from(plan.sample_rate)),
                self.limits.arithmetic_bits,
            )?;
        let voice = AudioSpanVoice {
            track: mo_video_model::TrackId::new("retime-input").expect("static ID"),
            clip: plan.clip.clone(),
            first_sample: mo_common::Ticks::new(first as i64),
            frames,
            local_start: R::integer(0),
            mapping: mo_video_model::MappedTime {
                sample: mo_video_model::TimeSample {
                    time: plan.input_zero.add(&offset, self.limits.arithmetic_bits)?,
                    side: if plan.direction < 0 {
                        SampleSide::Before
                    } else {
                        SampleSide::After
                    },
                },
                rate: R::integer(plan.direction),
            },
            policy: mo_video_model::AudioTimePolicy {
                rate: AudioRatePolicy::Resample,
                reverse: mo_video_model::ReverseAudioPolicy::Reverse,
                hold: mo_video_model::HoldAudioPolicy::Silence,
            },
            effects: vec![],
            source: plan.source.clone(),
        };
        let mut session = Session {
            mixer: self,
            resources,
            check,
            stats: MixStats::default(),
            active_bytes: 0,
        };
        let pcm = session.resampled_voice(&voice, plan.sample_rate, 1)?;
        session.stats.retained_kernel_bytes = session
            .mixer
            .kernels
            .iter()
            .map(|(_, k)| k.kernel_bytes())
            .sum();
        session.stats.retained_retime_plan_bytes = session.mixer.retime_plans.retained_bytes();
        cancel(check)?;
        Ok(MixedAudio {
            pcm,
            stats: session.stats,
        })
    }
}
struct Session<'a, 'd> {
    mixer: &'a mut AudioMixer<'d>,
    resources: &'a mut dyn AudioResources,
    check: &'a dyn Fn() -> bool,
    stats: MixStats,
    active_bytes: usize,
}
impl Session<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<()> {
        let total = self
            .active_bytes
            .checked_add(bytes)
            .ok_or(AudioError::Budget)?;
        if total > self.mixer.limits.max_active_pcm_bytes {
            return Err(AudioError::Budget);
        }
        self.active_bytes = total;
        self.stats.peak_pcm_bytes = self.stats.peak_pcm_bytes.max(total);
        Ok(())
    }
    fn pcm_limits(&self) -> PcmLimits {
        PcmLimits {
            max_frames: self.mixer.limits.pcm.max_frames,
            max_bytes: self
                .mixer
                .limits
                .pcm
                .max_bytes
                .min(self.mixer.limits.max_active_pcm_bytes - self.active_bytes),
        }
    }
    fn query_limits(&self) -> Result<QueryLimits> {
        let visits = self
            .mixer
            .limits
            .max_visits
            .checked_sub(self.stats.visited)
            .ok_or(AudioError::Budget)?;
        let voices = self
            .mixer
            .limits
            .max_voices
            .checked_sub(self.stats.voices)
            .ok_or(AudioError::Budget)?;
        if visits == 0 || voices == 0 {
            return Err(AudioError::Budget);
        }
        Ok(QueryLimits {
            max_visits: visits,
            max_nodes: voices,
            ..QueryLimits::default()
        })
    }
    fn kernel(&mut self, step: &R) -> Result<Arc<SincResampler>> {
        if let Some(index) = self
            .mixer
            .kernels
            .iter()
            .position(|(key, _)| key.compare(step).is_eq())
        {
            let entry = self.mixer.kernels.remove(index);
            let kernel = Arc::clone(&entry.1);
            self.mixer.kernels.push(entry);
            return Ok(kernel);
        }
        // Evict only unreferenced entries; an ancestor's active filter cannot be
        // hidden from the bound by removing its cache entry during recursion.
        let mut retained: usize = self
            .mixer
            .kernels
            .iter()
            .map(|(_, k)| k.kernel_bytes())
            .sum();
        let required =
            SincResampler::estimate_kernel_bytes(step, self.mixer.limits.arithmetic_bits)?;
        while retained
            .checked_add(required)
            .is_none_or(|n| n > self.mixer.limits.max_kernel_bytes)
        {
            let Some(index) = self
                .mixer
                .kernels
                .iter()
                .position(|(_, k)| Arc::strong_count(k) == 1)
            else {
                return Err(AudioError::Budget);
            };
            retained -= self.mixer.kernels.remove(index).1.kernel_bytes();
        }
        let kernel = Arc::new(SincResampler::new(
            step,
            self.mixer.limits.arithmetic_bits,
            ResampleLimits {
                max_kernel_bytes: self.mixer.limits.max_kernel_bytes - retained,
                ..ResampleLimits::default()
            },
            self.check,
        )?);
        self.mixer.kernels.push((step.clone(), Arc::clone(&kernel)));
        Ok(kernel)
    }
    fn sequence(
        &mut self,
        id: &SequenceId,
        first: i64,
        frames: u32,
        depth: usize,
    ) -> Result<PcmBlock> {
        cancel(self.check)?;
        if depth > self.mixer.limits.max_depth {
            return Err(AudioError::Budget);
        }
        let computed = self.mixer.document.audio_sequence_block(
            id,
            first,
            frames,
            self.query_limits()?,
            self.check,
        )?;
        self.stats.visited += computed.stats.visited;
        self.stats.voices += computed.stats.selected;
        let plan = computed.value;
        let count = crate::pcm::admit(
            plan.sample_rate,
            &self.mixer.output,
            frames as usize,
            self.mixer.limits.pcm,
        )?;
        self.reserve(count.checked_mul(12).ok_or(AudioError::Budget)?)?;
        let mut sum: Vec<f64> = Vec::new();
        sum.try_reserve_exact(count)
            .map_err(|_| AudioError::Budget)?;
        sum.resize(count, 0.);
        for voice in &plan.voices {
            let block = self.voice(voice, plan.sample_rate, depth)?;
            let steps = voice.frames as usize * voice.effects.len().max(1);
            self.stats.automation_steps = self
                .stats
                .automation_steps
                .checked_add(steps)
                .ok_or(AudioError::Budget)?;
            if self.stats.automation_steps > self.mixer.limits.max_automation_steps {
                return Err(AudioError::Budget);
            }
            let mut envelope =
                Envelope::new(voice, plan.sample_rate, self.mixer.limits.arithmetic_bits)?;
            let offset = (voice.first_sample.get() - first) as usize;
            let channels = self.mixer.output.len();
            for frame in 0..voice.frames as usize {
                if frame % 128 == 0 {
                    cancel(self.check)?;
                }
                let gain = envelope.value(frame)?;
                for ch in 0..channels {
                    sum[(offset + frame) * channels + ch] +=
                        f64::from(block.samples()[frame * channels + ch]) * gain;
                }
            }
            self.active_bytes -= block.bytes();
        }
        let mut samples = crate::pcm::allocate(count)?;
        for (i, (to, from)) in samples.iter_mut().zip(sum).enumerate() {
            if i % 4096 == 0 {
                cancel(self.check)?;
            }
            *to = from as f32;
            if !to.is_finite() {
                return Err(AudioError::Invalid("audio mix overflow"));
            }
        }
        self.active_bytes -= count * 8;
        PcmBlock::new(
            plan.sample_rate,
            self.mixer.output.clone(),
            first,
            samples,
            self.mixer.limits.pcm,
            self.check,
        )
    }
    fn voice(&mut self, voice: &AudioSpanVoice, rate: u32, depth: usize) -> Result<PcmBlock> {
        if voice.policy.rate == AudioRatePolicy::PreservePitch
            && let Some(request) = self.retime_request(voice)?
        {
            let first = voice
                .first_sample
                .get()
                .checked_sub(request.plan().first_sample())
                .ok_or(AudioError::Budget)?;
            let range = crate::PcmRange::new(
                first,
                first
                    .checked_add(i64::from(voice.frames))
                    .ok_or(AudioError::Budget)?,
            )?;
            let limits = self.pcm_limits();
            crate::pcm::admit(rate, &self.mixer.output, voice.frames as usize, limits)?;
            self.reserve(voice.frames as usize * self.mixer.output.len() * 4)?;
            let mut pcm = self
                .resources
                .read_retimed(&request, range, limits, self.check)?;
            cancel(self.check)?;
            if pcm.sample_rate() != rate
                || pcm.layout() != &self.mixer.output
                || pcm.first_frame() != range.first
                || pcm.end_frame() != range.end
            {
                return Err(AudioError::Invalid("derived audio window binding"));
            }
            pcm.first_frame = voice.first_sample.get();
            return Ok(pcm);
        }
        self.resampled_voice(voice, rate, depth)
    }
    fn retime_request(&mut self, voice: &AudioSpanVoice) -> Result<Option<crate::RetimeRequest>> {
        let first = voice.first_sample.get();
        let found = if let Some(found) = self.mixer.retime_plans.lookup(&voice.clip, first)? {
            found
        } else {
            let admission = crate::retime_plan::admit_clip(
                &self.mixer.document,
                &voice.clip,
                self.mixer.limits.arithmetic_bits,
                self.mixer
                    .limits
                    .max_visits
                    .saturating_sub(self.stats.visited),
                self.check,
            )?;
            self.stats.visited += admission.visits;
            self.mixer
                .retime_plans
                .admit(admission.charge, self.mixer.limits.max_retime_plan_bytes)?;
            let plans = crate::retime_plan::compile_clip(
                &self.mixer.document,
                &voice.source,
                admission,
                self.mixer.limits.arithmetic_bits,
                self.check,
            )?;
            self.mixer.retime_plans.insert(voice.clip.clone(), plans)?;
            self.mixer
                .retime_plans
                .lookup(&voice.clip, first)?
                .flatten()
        };
        found
            .map(|plan| {
                crate::RetimeRequest::new(
                    plan,
                    self.mixer.output.clone(),
                    self.resources.retime_profile()?,
                )
            })
            .transpose()
    }
    fn resampled_voice(
        &mut self,
        voice: &AudioSpanVoice,
        rate: u32,
        depth: usize,
    ) -> Result<PcmBlock> {
        let bits = self.mixer.limits.arithmetic_bits;
        let asset = match &voice.source {
            AudioSpanSource::Media { binding } => {
                let asset = self.resources.describe(binding)?;
                asset.validate_binding(binding, bits)?;
                Some(asset)
            }
            AudioSpanSource::Sequence { .. } => None,
        };
        let source_rate = asset.as_ref().map_or(rate, |a| a.sample_rate);
        let zero = asset
            .as_ref()
            .map_or(R::integer(0), |a| a.sample_zero.clone());
        let mut start = voice
            .mapping
            .sample
            .time
            .sub(&zero, bits)?
            .mul(&R::integer(i64::from(source_rate)), bits)?;
        // Discrete reversal acts on sample cells: boundary N before -> N-1.
        // This is an exact lattice convention, not a floating epsilon.
        if voice.mapping.sample.side == SampleSide::Before {
            start = start.sub(&R::integer(1), bits)?;
        }
        let step = voice
            .mapping
            .rate
            .mul(&R::integer(i64::from(source_rate)), bits)?
            .div(&R::integer(i64::from(rate)), bits)?;
        let kernel = self.kernel(&step)?;
        let needed = kernel.required_range(&start, voice.frames as usize)?;
        let (input, available) = match (&voice.source, asset) {
            (AudioSpanSource::Media { binding }, Some(asset)) => {
                let needed = needed.intersection(asset.available);
                let frames =
                    usize::try_from(needed.end - needed.first).map_err(|_| AudioError::Budget)?;
                let limits = self.pcm_limits();
                let count = crate::pcm::admit(asset.sample_rate, &asset.layout, frames, limits)?;
                self.reserve(count * 4)?;
                let input = if frames == 0 {
                    PcmBlock::new(
                        asset.sample_rate,
                        asset.layout.clone(),
                        needed.first,
                        vec![],
                        limits,
                        self.check,
                    )?
                } else {
                    self.resources
                        .read(binding, &asset, needed, limits, self.check)?
                };
                cancel(self.check)?;
                if input.first_frame() != needed.first
                    || input.end_frame() != needed.end
                    || input.layout() != &asset.layout
                    || input.sample_rate() != asset.sample_rate
                {
                    return Err(AudioError::Invalid("audio provider window binding"));
                }
                self.stats.source_frames = self
                    .stats
                    .source_frames
                    .checked_add(frames)
                    .ok_or(AudioError::Budget)?;
                let input = self.channels(input)?;
                (input, asset.available)
            }
            (AudioSpanSource::Sequence { sequence }, None) => {
                let frames =
                    usize::try_from(needed.end - needed.first).map_err(|_| AudioError::Budget)?;
                let count = crate::pcm::admit(rate, &self.mixer.output, frames, self.pcm_limits())?;
                self.reserve(count * 4)?;
                let mut samples = crate::pcm::allocate(count)?;
                let channels = self.mixer.output.len();
                let mut first = needed.first;
                while first < needed.end {
                    let n = (needed.end - first).min(65536) as u32;
                    let block = self.sequence(sequence, first, n, depth + 1)?;
                    let offset = (first - needed.first) as usize * channels;
                    samples[offset..offset + block.samples().len()]
                        .copy_from_slice(block.samples());
                    self.active_bytes -= block.bytes();
                    first += i64::from(n);
                }
                (
                    PcmBlock::new(
                        rate,
                        self.mixer.output.clone(),
                        needed.first,
                        samples,
                        self.mixer.limits.pcm,
                        self.check,
                    )?,
                    needed,
                )
            }
            _ => return Err(AudioError::Invalid("audio source binding")),
        };
        let ops = kernel.multiply_adds(&start, voice.frames as usize, self.mixer.output.len())?;
        self.stats.multiply_adds = self
            .stats
            .multiply_adds
            .checked_add(ops)
            .ok_or(AudioError::Budget)?;
        if self.stats.multiply_adds > self.mixer.limits.max_multiply_adds {
            return Err(AudioError::Budget);
        }
        self.reserve(voice.frames as usize * self.mixer.output.len() * 4)?;
        let pcm = kernel.process(
            &input,
            available,
            &start,
            rate,
            voice.first_sample.get(),
            voice.frames as usize,
            ResampleLimits {
                pcm: self.mixer.limits.pcm,
                max_kernel_bytes: self.mixer.limits.max_kernel_bytes,
                max_multiply_adds: ops,
                ..ResampleLimits::default()
            },
            self.check,
        )?;
        self.active_bytes -= input.bytes();
        Ok(pcm)
    }
    fn channels(&mut self, input: PcmBlock) -> Result<PcmBlock> {
        if input.layout() == &self.mixer.output {
            return Ok(input);
        }
        let matrix = self
            .resources
            .channel_matrix(input.layout(), &self.mixer.output)?;
        if matrix.output() != &self.mixer.output {
            return Err(AudioError::Invalid("audio channel matrix output binding"));
        }
        let limits = self.pcm_limits();
        let count = crate::pcm::admit(
            input.sample_rate(),
            &self.mixer.output,
            input.frames(),
            limits,
        )?;
        let ops = count
            .checked_mul(input.layout().len())
            .ok_or(AudioError::Budget)?;
        self.stats.multiply_adds = self
            .stats
            .multiply_adds
            .checked_add(ops)
            .ok_or(AudioError::Budget)?;
        if self.stats.multiply_adds > self.mixer.limits.max_multiply_adds {
            return Err(AudioError::Budget);
        }
        self.reserve(count * 4)?;
        let mapped = input.map_channels(&matrix, limits, self.check)?;
        self.active_bytes -= input.bytes();
        Ok(mapped)
    }
}
