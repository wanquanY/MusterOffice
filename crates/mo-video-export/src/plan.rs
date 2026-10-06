use mo_media::{EncodeConfig, EncodeProfile};
use mo_time::ExactRatio as R;
use mo_video_audio::{ChannelLayout, MixLimits};
use mo_video_compile::{CompiledDocument, QueryLimits};
use mo_video_render::{I420Options, RenderLimits, Sampling, ToneMap};

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("invalid export configuration: {0}")]
    Invalid(&'static str),
    #[error("export exceeds configured limits")]
    Budget,
    #[error("export planning cancelled")]
    Cancelled,
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
    #[error(transparent)]
    Compile(#[from] mo_video_compile::CompileError),
}
#[derive(Debug, Clone, Copy)]
pub enum ExportAudio {
    Omit,
    Mono { bit_rate: u32 },
    Stereo { bit_rate: u32 },
}
impl ExportAudio {
    pub(crate) fn layout(self) -> Option<ChannelLayout> {
        match self {
            Self::Omit => None,
            Self::Mono { .. } => Some(ChannelLayout::mono()),
            Self::Stereo { .. } => Some(ChannelLayout::stereo()),
        }
    }
    fn rate(self) -> u32 {
        match self {
            Self::Omit => 0,
            Self::Mono { bit_rate } | Self::Stereo { bit_rate } => bit_rate,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ExportOptions {
    pub profile: EncodeProfile,
    pub video_bit_rate: u32,
    pub gop_frames: u32,
    pub audio: ExportAudio,
    pub background_srgb: [u8; 3],
    pub tone_map: Option<ToneMap>,
    pub sampling: Sampling,
}
#[derive(Debug, Clone, Copy)]
pub struct ExportLimits {
    pub max_video_frames: u32,
    pub max_audio_samples: i64,
    /// Current final linear image + owned I420 + conversion scratch. Resolver,
    /// composition intermediates, mixer caches and codec memory are separate.
    pub max_video_output_bytes: usize,
    pub audio_block_frames: u32,
    pub query: QueryLimits,
    pub render: RenderLimits,
    pub mix: MixLimits,
}
impl Default for ExportLimits {
    fn default() -> Self {
        Self {
            max_video_frames: 10_000_000,
            max_audio_samples: 48_000 * 60 * 60 * 24,
            max_video_output_bytes: 512 << 20,
            audio_block_frames: 1024,
            query: QueryLimits::default(),
            render: RenderLimits::default(),
            mix: MixLimits::default(),
        }
    }
}
pub struct ExportPlan<'a> {
    pub(crate) document: &'a CompiledDocument,
    pub(crate) options: ExportOptions,
    pub(crate) limits: ExportLimits,
    pub(crate) config: EncodeConfig,
    pub(crate) period: i64,
    pub(crate) output: I420Options,
    audio_tail: R,
}
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
fn lcm(a: u64, b: u64) -> Result<u64, PlanError> {
    (a / gcd(a, b)).checked_mul(b).ok_or(PlanError::Budget)
}
impl<'a> ExportPlan<'a> {
    pub fn new(
        document: &'a CompiledDocument,
        options: ExportOptions,
        limits: ExportLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PlanError> {
        if check() {
            return Err(PlanError::Cancelled);
        }
        let d = document.document();
        if !document.duration().positive() {
            return Err(PlanError::Invalid("empty project"));
        }
        if !(1..=65536).contains(&limits.audio_block_frames)
            || limits.max_audio_samples < 0
            || !(10000..=1_000_000_000).contains(&options.video_bit_rate)
            || !(1..=10000).contains(&options.gop_frames)
            || !(2..=8192).contains(&d.canvas.width)
            || !(2..=8192).contains(&d.canvas.height)
            || !d.canvas.width.is_multiple_of(2)
            || !d.canvas.height.is_multiple_of(2)
        {
            return Err(PlanError::Invalid("dimensions, bitrate, GOP or block size"));
        }
        let rate = d.frame_rate;
        if u64::from(rate.numerator) < u64::from(rate.denominator)
            || u64::from(rate.numerator) > 240 * u64::from(rate.denominator)
        {
            return Err(PlanError::Invalid("output frame rate outside profile"));
        }
        let duration = document.duration().to_time_exact()?;
        let period = rate.frame_time(1, 256)?.to_time_exact()?;
        let scale = lcm(
            u64::from(duration.timescale.get()),
            u64::from(period.timescale.get()),
        )?;
        if scale > 1_000_000_000 {
            return Err(PlanError::Invalid("exact video clock cannot fit profile"));
        }
        let video_duration = duration
            .ticks
            .get()
            .checked_mul((scale / u64::from(duration.timescale.get())) as i64)
            .ok_or(PlanError::Budget)?;
        let period = period
            .ticks
            .get()
            .checked_mul((scale / u64::from(period.timescale.get())) as i64)
            .ok_or(PlanError::Budget)?;
        let frames = u32::try_from(document.frame_count()?).map_err(|_| PlanError::Budget)?;
        if frames > limits.max_video_frames {
            return Err(PlanError::Budget);
        }
        let layout = options.audio.layout();
        let samples = if layout.is_some() {
            document.audio_sample_count()?
        } else {
            0
        };
        if samples > limits.max_audio_samples {
            return Err(PlanError::Budget);
        }
        if layout.is_some()
            && (!(8000..=192000).contains(&d.audio_sample_rate)
                || !(8000..=512000).contains(&options.audio.rate()))
        {
            return Err(PlanError::Invalid("audio encoder profile"));
        }
        let movie_scale = if samples == 0 {
            scale
        } else {
            lcm(scale, u64::from(d.audio_sample_rate))?
        };
        if movie_scale > i32::MAX as u64
            || i128::from(video_duration) * i128::from(movie_scale / scale) > i128::from(i64::MAX)
            || (samples > 0
                && i128::from(samples) * i128::from(movie_scale / u64::from(d.audio_sample_rate))
                    > i128::from(i64::MAX))
        {
            return Err(PlanError::Invalid("exact movie clock cannot fit profile"));
        }
        let audio_tail = if samples == 0 {
            R::integer(0)
        } else {
            R::integer(samples)
                .div(&R::integer(i64::from(d.audio_sample_rate)), 256)?
                .sub(document.duration(), 256)?
        };
        let count = (u64::from(d.canvas.width) * u64::from(d.canvas.height)) as usize;
        let output_bytes = count + count / 2;
        let scratch = d.canvas.width as usize * 80;
        if count
            .checked_mul(16)
            .and_then(|n| n.checked_add(output_bytes))
            .and_then(|n| n.checked_add(scratch))
            .is_none_or(|n| n > limits.max_video_output_bytes)
        {
            return Err(PlanError::Budget);
        }
        if check() {
            return Err(PlanError::Cancelled);
        }
        Ok(Self {
            document,
            options,
            limits,
            period,
            audio_tail,
            output: I420Options {
                background_srgb: options.background_srgb,
                tone_map: options.tone_map,
                max_bytes: output_bytes,
                max_temporary_bytes: scratch,
            },
            config: EncodeConfig {
                profile: options.profile,
                width: d.canvas.width,
                height: d.canvas.height,
                video_time_scale: scale as u32,
                frame_rate_num: rate.numerator,
                frame_rate_den: rate.denominator,
                video_duration,
                video_frames: frames,
                video_bit_rate: options.video_bit_rate,
                gop_frames: options.gop_frames,
                audio_sample_rate: d.audio_sample_rate,
                audio_channels: layout.as_ref().map_or(0, |v| v.len() as u32),
                audio_bit_rate: options.audio.rate(),
                audio_samples: samples,
            },
        })
    }
    pub fn configuration(&self) -> EncodeConfig {
        self.config
    }
    pub fn document_digest(&self) -> &mo_common::Digest {
        self.document.digest()
    }
    /// Audio's indivisible final sample extends at most one sample beyond the
    /// exact authoring end. Retain that positive residual for host verification.
    pub fn audio_tail(&self) -> &R {
        &self.audio_tail
    }
}
