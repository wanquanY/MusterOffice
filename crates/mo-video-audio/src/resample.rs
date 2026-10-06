//! Deterministic, zero-phase, band-limited sample-rate conversion. Required
//! pre/post-roll is explicit; missing samples inside the asset are an error.
use crate::{AudioError, PcmBlock, PcmLimits, PhaseClock, Result, cancel};
use mo_time::{ExactRatio as R, Rounding};

const PHASES: usize = 1024;
const HALF_WIDTH: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmRange {
    pub first: i64,
    pub end: i64,
}
impl PcmRange {
    pub fn new(first: i64, end: i64) -> Result<Self> {
        if first > end {
            return Err(AudioError::Invalid("PCM range"));
        }
        Ok(Self { first, end })
    }
    pub fn intersection(self, other: Self) -> Self {
        let first = self.first.max(other.first);
        Self {
            first,
            end: self.end.min(other.end).max(first),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ResampleLimits {
    pub pcm: PcmLimits,
    pub max_kernel_bytes: usize,
    pub max_multiply_adds: usize,
    pub max_radius: usize,
}
impl Default for ResampleLimits {
    fn default() -> Self {
        Self {
            pcm: PcmLimits::default(),
            max_kernel_bytes: 32 << 20,
            max_multiply_adds: 64_000_000,
            max_radius: 4096,
        }
    }
}
#[derive(Debug)]
pub struct SincResampler {
    step: R,
    bits: u64,
    radius: usize,
    weights: Vec<f64>,
    unit_step: bool,
}
impl SincResampler {
    fn shape(step: &R, bits: u64) -> Result<(usize, f64)> {
        step.check_budget(bits)?;
        let magnitude = if step.compare(&R::integer(0)).is_lt() {
            R::integer(0).sub(step, bits)?
        } else {
            step.clone()
        };
        if magnitude.compare(&R::integer(0)).is_eq() {
            return Err(AudioError::Invalid("zero resampling step"));
        }
        if magnitude.compare(&R::integer(64)).is_gt() {
            return Err(AudioError::Budget);
        }
        let speed = PhaseClock::new(&magnitude, &R::integer(0), bits)?
            .value()
            .max(1.);
        let radius = if magnitude.compare(&R::integer(1)).is_le() {
            HALF_WIDTH
        } else {
            magnitude
                .mul(&R::integer(HALF_WIDTH as i64), bits)?
                .round_integer(Rounding::Ceil)? as usize
        };
        Ok((radius, 0.94 / speed))
    }
    pub fn estimate_kernel_bytes(step: &R, bits: u64) -> Result<usize> {
        let (radius, _) = Self::shape(step, bits)?;
        Ok((PHASES + 1) * 2 * radius * 8)
    }
    pub fn multiply_adds(&self, start: &R, frames: usize, channels: usize) -> Result<usize> {
        frames
            .checked_mul(channels)
            .and_then(|n| {
                n.checked_mul(if self.identity(start) {
                    1
                } else {
                    2 * self.radius
                })
            })
            .ok_or(AudioError::Budget)
    }
    pub fn new(
        step: &R,
        bits: u64,
        limits: ResampleLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        cancel(check)?;
        let (radius, cutoff) = Self::shape(step, bits)?;
        if radius > limits.max_radius || radius > 4096 {
            return Err(AudioError::Budget);
        }
        let count = (PHASES + 1) * 2 * radius;
        let bytes = count * 8;
        if bytes > limits.max_kernel_bytes || bytes > 128 << 20 {
            return Err(AudioError::Budget);
        }
        let mut weights = Vec::new();
        weights
            .try_reserve_exact(count)
            .map_err(|_| AudioError::Budget)?;
        for phase in 0..=PHASES {
            cancel(check)?;
            let fraction = phase as f64 / PHASES as f64;
            let begin = weights.len();
            for tap in 0..2 * radius {
                let distance = (tap as i64 - radius as i64 + 1) as f64 - fraction;
                let u = distance / radius as f64;
                let window = if u.abs() >= 1. {
                    0.
                } else {
                    0.35875
                        + 0.48829 * libm::cos(std::f64::consts::PI * u)
                        + 0.14128 * libm::cos(2. * std::f64::consts::PI * u)
                        + 0.01168 * libm::cos(3. * std::f64::consts::PI * u)
                };
                let x = std::f64::consts::PI * cutoff * distance;
                weights.push(cutoff * if x == 0. { 1. } else { libm::sin(x) / x } * window);
            }
            let sum: f64 = weights[begin..].iter().sum();
            for weight in &mut weights[begin..] {
                *weight /= sum;
            }
        }
        Ok(Self {
            step: step.clone(),
            bits,
            radius,
            weights,
            unit_step: step.compare(&R::integer(1)).is_eq()
                || step.compare(&R::integer(-1)).is_eq(),
        })
    }
    pub fn kernel_bytes(&self) -> usize {
        self.weights.len() * 8
    }
    pub fn radius(&self) -> usize {
        self.radius
    }
    fn identity(&self, start: &R) -> bool {
        self.unit_step && start.normalized().denominator() == &1.into()
    }
    /// Conservative contiguous source footprint. The adapter intersects it with
    /// the known asset range and must supply every sample in that intersection.
    pub fn required_range(&self, start: &R, frames: usize) -> Result<PcmRange> {
        if frames == 0 || frames > 65536 {
            return Err(AudioError::Invalid("resampling frame count"));
        }
        let last = start.add(
            &self.step.mul(&R::integer((frames - 1) as i64), self.bits)?,
            self.bits,
        )?;
        let a = start.round_integer(Rounding::Floor)?;
        let b = last.round_integer(Rounding::Floor)?;
        let (before, after) = if self.identity(start) {
            (0, 1)
        } else {
            (self.radius as i64 - 1, self.radius as i64 + 1)
        };
        PcmRange::new(
            a.min(b).checked_sub(before).ok_or(AudioError::Budget)?,
            a.max(b).checked_add(after).ok_or(AudioError::Budget)?,
        )
    }
    /// `available` is the complete continuous asset span, not a cache window.
    /// Outside it is zero extension; missing data inside it is never silence.
    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &self,
        input: &PcmBlock,
        available: PcmRange,
        start: &R,
        output_rate: u32,
        output_first: i64,
        frames: usize,
        limits: ResampleLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        cancel(check)?;
        PcmRange::new(available.first, available.end)?;
        let needed = self.required_range(start, frames)?.intersection(available);
        if needed.first < needed.end
            && (input.first_frame() > needed.first || input.end_frame() < needed.end)
        {
            return Err(AudioError::Resource("incomplete PCM footprint".into()));
        }
        if input.frames() != 0
            && (input.first_frame() < available.first || input.end_frame() > available.end)
        {
            return Err(AudioError::Invalid("PCM outside asset span"));
        }
        let count = crate::pcm::admit(output_rate, input.layout(), frames, limits.pcm)?;
        let identity = self.identity(start);
        let taps = if identity { 1 } else { 2 * self.radius };
        if count
            .checked_mul(taps)
            .is_none_or(|n| n > limits.max_multiply_adds || n > 1_000_000_000)
        {
            return Err(AudioError::Budget);
        }
        if self.kernel_bytes() > limits.max_kernel_bytes || self.radius > limits.max_radius {
            return Err(AudioError::Budget);
        }
        let mut samples = crate::pcm::allocate(count)?;
        let mut clock = PhaseClock::new(start, &self.step, self.bits)?;
        let channels = input.layout().len();
        for frame in 0..frames {
            if frame % 32 == 0 {
                cancel(check)?;
            }
            let (whole, fraction) = clock.position();
            if identity {
                if whole >= available.first && whole < available.end {
                    let offset = (whole - input.first_frame()) as usize * channels;
                    samples[frame * channels..(frame + 1) * channels]
                        .copy_from_slice(&input.samples()[offset..offset + channels]);
                }
            } else {
                let phase = (fraction >> 22) as usize;
                let mix = f64::from(fraction & ((1 << 22) - 1)) / 4194304.;
                let base = whole - self.radius as i64 + 1; // checked by required_range
                for ch in 0..channels {
                    if ch % 8 == 0 {
                        cancel(check)?;
                    }
                    let mut sum = 0.;
                    for tap in 0..taps {
                        let source = base + tap as i64;
                        if source >= available.first && source < available.end {
                            let w0 = self.weights[phase * taps + tap];
                            let w1 = self.weights[(phase + 1) * taps + tap];
                            sum += f64::from(
                                input.samples()
                                    [(source - input.first_frame()) as usize * channels + ch],
                            ) * (w0 + (w1 - w0) * mix);
                        }
                    }
                    let value = sum as f32;
                    if !value.is_finite() {
                        return Err(AudioError::Invalid("resampling overflow"));
                    }
                    samples[frame * channels + ch] = value;
                }
            }
            if frame + 1 < frames {
                clock.advance()?;
            }
        }
        PcmBlock::new(
            output_rate,
            input.layout().clone(),
            output_first,
            samples,
            limits.pcm,
            check,
        )
    }
}
