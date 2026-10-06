//! Canonical, sequential pitch-preserving stream. Chunk boundaries are part of
//! the processing profile, never chosen by UI/decoder/audio-device callbacks.
use crate::{AudioError, ChannelLayout, PcmBlock, PcmLimits, Result, cancel};

pub const STRETCH_QUANTUM: u32 = 128;
pub const STRETCH_PROFILE: &str = "musteroffice.audio.stretch-sequential-q128/1-draft";

#[derive(Debug, Clone, Copy)]
pub struct StretchInfo {
    pub channels: usize,
    pub sample_rate: u32,
    pub input_latency: u32,
    pub output_latency: u32,
}
/// Narrow DSP capability. The implementation owns finite validation, bounded
/// allocation, and discards its state after a processing failure. Calls are
/// serial; the caller must never run expensive DSP on a device callback.
pub trait StretchEngine {
    fn info(&self) -> StretchInfo;
    fn begin(&mut self, input: &[f32], rate: f64) -> Result<()>;
    fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<()>;
}
/// The host fills the entire requested interleaved window, at the stream's
/// sample rate/layout. It may read canonical PCM or render a child sequence.
/// Missing interior data returns PendingSource, never implicit silence. The
/// stream alone pads beyond the declared finite source.
pub trait StretchInput {
    fn read(&mut self, first: u64, samples: &mut [f32], check: &dyn Fn() -> bool) -> Result<()>;
}
#[derive(Debug, Clone, Copy)]
pub struct StretchLimits {
    /// Rust scratch + candidate output payload; DSP memory is a separate bound
    /// reported by the engine. Host/allocator overhead is not included.
    pub max_buffer_bytes: usize,
    pub max_preroll_calls: usize,
}
impl Default for StretchLimits {
    fn default() -> Self {
        Self {
            max_buffer_bytes: 32 << 20,
            max_preroll_calls: 1024,
        }
    }
}
/// Exact integer duration ratio; input and output clocks never accumulate floats.
/// Each stream starts at source frame zero. Reverse/rate-stage construction and
/// persistent random-access tiles belong to the graph/host, not this DSP.
pub struct StretchStream<E> {
    engine: E,
    info: StretchInfo,
    layout: ChannelLayout,
    sample_rate: u32,
    source_frames: u64,
    output_frames: u64,
    processed: u64,
    begun: bool,
    poisoned: bool,
    input: Vec<f32>,
    output: Vec<f32>,
    limits: StretchLimits,
}
impl<E: StretchEngine> StretchStream<E> {
    pub fn new(
        engine: E,
        sample_rate: u32,
        layout: ChannelLayout,
        source_frames: u64,
        output_frames: u64,
        limits: StretchLimits,
    ) -> Result<Self> {
        let info = engine.info();
        if !(8000..=384000).contains(&sample_rate)
            || info.sample_rate != sample_rate
            || info.channels != layout.len()
            || info.input_latency == 0
            || info.input_latency > 65536
            || info.output_latency == 0
            || info.output_latency > 65536
            || source_frames == 0
            || output_frames == 0
            || source_frames > i64::MAX as u64
            || output_frames > i64::MAX as u64
            || u128::from(source_frames) > u128::from(output_frames) * 64
            || u128::from(output_frames) > u128::from(source_frames) * 64
            || limits.max_buffer_bytes > 128 << 20
            || limits.max_preroll_calls == 0
        {
            return Err(AudioError::Invalid("stretch stream configuration"));
        }
        let count = (info.input_latency as usize).max(STRETCH_QUANTUM as usize * 64) * layout.len();
        let out_count = STRETCH_QUANTUM as usize * layout.len();
        // Include one returned candidate; previously returned outputs are host-owned.
        if (count + out_count * 2) * 4 > limits.max_buffer_bytes {
            return Err(AudioError::Budget);
        }
        if limits.max_preroll_calls <= info.output_latency as usize / STRETCH_QUANTUM as usize {
            return Err(AudioError::Budget);
        }
        let input = crate::pcm::allocate(count)?;
        let output = crate::pcm::allocate(out_count)?;
        Ok(Self {
            engine,
            info,
            layout,
            sample_rate,
            source_frames,
            output_frames,
            processed: 0,
            begun: false,
            poisoned: false,
            input,
            output,
            limits,
        })
    }
    pub fn buffer_bytes(&self) -> usize {
        (self.input.len() + self.output.len()) * 4
    }
    pub fn engine(&self) -> &E {
        &self.engine
    }
    /// Emits the next canonical chunk, removing output latency and retaining
    /// exactly output_frames samples. Long and sub-window clips use the same
    /// explicit zero-extension rule. No upstream short-buffer silent fallback.
    ///
    /// A source-read failure is retryable because no corresponding DSP step was
    /// performed. Once a DSP step fails or cancellation follows state mutation,
    /// the stream is poisoned. A host publishes each successful chunk atomically.
    pub fn next(
        &mut self,
        source: &mut dyn StretchInput,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<PcmBlock>> {
        if self.poisoned {
            return Err(AudioError::Invalid("poisoned stretch stream"));
        }
        cancel(check)?;
        if !self.begun {
            self.read(source, 0, self.info.input_latency as usize, check)?;
            let n = self.info.input_latency as usize * self.layout.len();
            // Also fail closed if a custom host engine panics and its caller
            // catches the unwind. A mutation cannot leave a reusable stream.
            self.poisoned = true;
            let rate_q32 = (u128::from(self.source_frames) << 32) / u128::from(self.output_frames);
            if let Err(error) = self
                .engine
                .begin(&self.input[..n], rate_q32 as f64 / 4294967296.)
            {
                self.poisoned = true;
                return Err(error);
            }
            self.begun = true;
            self.poisoned = false;
        }
        let delay = u64::from(self.info.output_latency);
        let end = self.output_frames + delay;
        for _ in 0..self.limits.max_preroll_calls {
            cancel(check)?;
            if self.processed == end {
                return Ok(None);
            }
            let next = (self.processed + u64::from(STRETCH_QUANTUM)).min(end);
            // Products fit u128 because all operands are <= i64::MAX+65536.
            let position = |frame: u64| -> u64 {
                u64::from(self.info.input_latency)
                    + (u128::from(frame) * u128::from(self.source_frames)
                        / u128::from(self.output_frames)) as u64
            };
            let first = position(self.processed);
            let in_frames = (position(next) - first) as usize;
            let out_frames = (next - self.processed) as usize;
            self.read(source, first, in_frames, check)?;
            let channels = self.layout.len();
            self.poisoned = true;
            let result = self.engine.process(
                &self.input[..in_frames * channels],
                &mut self.output[..out_frames * channels],
            );
            if let Err(error) = result {
                self.poisoned = true;
                return Err(error);
            }
            let previous = self.processed;
            self.processed = next;
            let cancelled = check();
            if cancelled
                || self.output[..out_frames * channels]
                    .iter()
                    .any(|v| !v.is_finite())
            {
                self.poisoned = true;
                return Err(if cancelled {
                    AudioError::Cancelled
                } else {
                    AudioError::Invalid("non-finite stretch output")
                });
            }
            if next > delay {
                let skip = (delay.saturating_sub(previous)) as usize;
                let samples = &self.output[skip * channels..out_frames * channels];
                let mut copy =
                    crate::pcm::allocate(samples.len()).inspect_err(|_| self.poisoned = true)?;
                copy.copy_from_slice(samples);
                let block = PcmBlock::new(
                    self.sample_rate,
                    self.layout.clone(),
                    previous.saturating_sub(delay) as i64,
                    copy,
                    PcmLimits {
                        max_frames: STRETCH_QUANTUM as usize,
                        max_bytes: self.limits.max_buffer_bytes,
                    },
                    check,
                )
                .inspect_err(|_| self.poisoned = true)?;
                self.poisoned = false;
                return Ok(Some(block));
            }
            self.poisoned = false;
        }
        Err(AudioError::Invalid("inconsistent stretch preroll schedule"))
    }
    fn read(
        &mut self,
        source: &mut dyn StretchInput,
        first: u64,
        frames: usize,
        check: &dyn Fn() -> bool,
    ) -> Result<()> {
        cancel(check)?;
        let count = frames * self.layout.len();
        self.input[..count].fill(0.);
        let real = self.source_frames.saturating_sub(first).min(frames as u64) as usize;
        if real > 0 {
            source.read(first, &mut self.input[..real * self.layout.len()], check)?;
        }
        cancel(check)?;
        if self.input[..count].iter().any(|v| !v.is_finite()) {
            return Err(AudioError::Invalid("non-finite stretch source"));
        }
        Ok(())
    }
}
