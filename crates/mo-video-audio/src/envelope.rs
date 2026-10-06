//! Sample-local automation clocks start from the authored phase, not block zero.
use crate::{PhaseClock, Result};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_compile::{AudioSpanEffect, AudioSpanVoice};
struct Ramp {
    first: usize,
    end: usize,
    clock: PhaseClock,
}
impl Ramp {
    fn value(&mut self, index: usize) -> Result<f64> {
        if index < self.first || index >= self.end {
            return Ok(1.);
        }
        let value = self.clock.value().clamp(0., 1.);
        if index + 1 < self.end {
            self.clock.advance()?;
        }
        Ok(value)
    }
}
enum Stage {
    Gain(f64),
    Fade {
        first: usize,
        end: usize,
        ramps: Vec<Ramp>,
    },
}
pub(crate) struct Envelope {
    stages: Vec<Stage>,
}
fn cut(value: &R, frames: usize) -> Result<usize> {
    if value.compare(&R::integer(0)).is_le() {
        Ok(0)
    } else if value.compare(&R::integer(frames as i64)).is_ge() {
        Ok(frames)
    } else {
        Ok(value.round_integer(Rounding::Ceil)? as usize)
    }
}
impl Envelope {
    pub fn new(voice: &AudioSpanVoice, sample_rate: u32, bits: u64) -> Result<Self> {
        let frames = voice.frames as usize;
        let sr = R::integer(i64::from(sample_rate));
        let mut stages = Vec::new();
        for effect in &voice.effects {
            stages.push(match effect {
                AudioSpanEffect::Gain { milli_decibels } => {
                    Stage::Gain(libm::pow(10., f64::from(*milli_decibels) / 20000.))
                }
                AudioSpanEffect::Fade {
                    fade_in,
                    fade_out,
                    envelope,
                } => {
                    let start = R::from_time(envelope.start);
                    let end = R::from_time(envelope.end);
                    let first = cut(
                        &start.sub(&voice.local_start, bits)?.mul(&sr, bits)?,
                        frames,
                    )?;
                    let last = cut(&end.sub(&voice.local_start, bits)?.mul(&sr, bits)?, frames)?;
                    let mut ramps = Vec::new();
                    for (duration, reverse) in [(*fade_in, false), (*fade_out, true)] {
                        if duration.ticks.get() == 0 {
                            continue;
                        }
                        let duration = R::from_time(duration);
                        let (a, b) = if reverse {
                            (end.sub(&duration, bits)?, end.clone())
                        } else {
                            (start.clone(), start.add(&duration, bits)?)
                        };
                        let lo = cut(&a.sub(&voice.local_start, bits)?.mul(&sr, bits)?, frames)?
                            .max(first);
                        let hi = cut(&b.sub(&voice.local_start, bits)?.mul(&sr, bits)?, frames)?
                            .min(last);
                        if lo >= hi {
                            continue;
                        }
                        let local = voice
                            .local_start
                            .add(&R::integer(lo as i64).div(&sr, bits)?, bits)?;
                        let phase = if reverse {
                            end.sub(&local, bits)?
                        } else {
                            local.sub(&start, bits)?
                        }
                        .div(&duration, bits)?;
                        let step = R::integer(if reverse { -1 } else { 1 })
                            .div(&sr.mul(&duration, bits)?, bits)?;
                        ramps.push(Ramp {
                            first: lo,
                            end: hi,
                            clock: PhaseClock::new(&phase, &step, bits)?,
                        });
                    }
                    Stage::Fade {
                        first,
                        end: last,
                        ramps,
                    }
                }
            });
        }
        Ok(Self { stages })
    }
    pub fn value(&mut self, index: usize) -> Result<f64> {
        let mut gain = 1.;
        for stage in &mut self.stages {
            gain *= match stage {
                Stage::Gain(v) => *v,
                Stage::Fade { first, end, ramps } => {
                    let mut factor = if index >= *first && index < *end {
                        1f64
                    } else {
                        0.
                    };
                    for ramp in ramps {
                        factor = factor.min(ramp.value(index)?);
                    }
                    factor
                }
            };
        }
        Ok(gain)
    }
}
