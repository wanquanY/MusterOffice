use crate::{AudioEffect, CompileError, Result, VisualEffect, Work};
use mo_time::ExactRatio as R;
use mo_video_model::{Effect, TimeSample};

fn fade(effect: &Effect, local: &TimeSample, bits: u64) -> Result<R> {
    let Effect::Fade {
        fade_in,
        fade_out,
        envelope,
    } = effect
    else {
        return Err(CompileError::Invariant("fade effect"));
    };
    if !local.covers(*envelope) {
        return Ok(R::integer(0));
    }
    let elapsed = local.time.sub(&R::from_time(envelope.start), bits)?;
    let remaining = R::from_time(envelope.end).sub(&local.time, bits)?;
    let mut factor = R::integer(1);
    for (position, duration) in [(elapsed, *fade_in), (remaining, *fade_out)] {
        // Outside the envelope is transparent/silent, including zero-length ramps.
        if position.compare(&R::integer(0)).is_lt() {
            return Ok(R::integer(0));
        }
        if duration.ticks.get() > 0 {
            let value = position.div(&R::from_time(duration), bits)?;
            if value.compare(&factor).is_lt() {
                factor = value;
            }
        }
    }
    Ok(factor)
}

pub(crate) fn visual(
    effects: &[Effect],
    local: &TimeSample,
    bits: u64,
    work: &mut Work<'_>,
) -> Result<Vec<VisualEffect>> {
    let mut output = vec![];
    for effect in effects {
        work.visit()?;
        work.bytes(256 + bits as usize)?;
        output.push(match effect {
            Effect::Transform {
                x_milli_pixels,
                y_milli_pixels,
                scale_x_ppm,
                scale_y_ppm,
                rotation_milli_degrees,
                opacity_ppm,
            } => VisualEffect::Transform {
                x_milli_pixels: *x_milli_pixels,
                y_milli_pixels: *y_milli_pixels,
                scale_x_ppm: *scale_x_ppm,
                scale_y_ppm: *scale_y_ppm,
                rotation_milli_degrees: *rotation_milli_degrees,
                opacity_ppm: *opacity_ppm,
            },
            Effect::Fade { .. } => VisualEffect::Opacity {
                factor: fade(effect, local, bits)?,
            },
            Effect::Gain { .. } => {
                return Err(CompileError::Invariant("audio gain on visual track"));
            }
        });
    }
    Ok(output)
}

pub(crate) fn audio(
    effects: &[Effect],
    local: &TimeSample,
    bits: u64,
    work: &mut Work<'_>,
) -> Result<Vec<AudioEffect>> {
    let mut output = vec![];
    for effect in effects {
        work.visit()?;
        work.bytes(256 + bits as usize)?;
        output.push(match effect {
            Effect::Gain { milli_decibels } => AudioEffect::Gain {
                milli_decibels: *milli_decibels,
            },
            Effect::Fade { .. } => AudioEffect::Amplitude {
                factor: fade(effect, local, bits)?,
            },
            Effect::Transform { .. } => {
                return Err(CompileError::Invariant("visual transform on audio track"));
            }
        });
    }
    Ok(output)
}
