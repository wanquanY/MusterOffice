use mo_common::RationalTime;
use mo_media::TimeBase;
use mo_time::{ExactRatio, Rounding, TimeError};
use mo_video_model::{SampleSide, TimeSample};

fn tick(base: TimeBase, bits: u64) -> Result<ExactRatio, TimeError> {
    if base.numerator == 0 || base.denominator == 0 {
        return Err(TimeError::Denominator);
    }
    ExactRatio::integer(i64::from(base.numerator))
        .div(&ExactRatio::integer(i64::from(base.denominator)), bits)
}
pub(crate) fn timestamp(ticks: i64, base: TimeBase, bits: u64) -> Result<RationalTime, TimeError> {
    ExactRatio::integer(ticks)
        .mul(&tick(base, bits)?, bits)?
        .to_time_exact()
}
pub(crate) fn seek_tick(sample: &TimeSample, base: TimeBase, bits: u64) -> Result<i64, TimeError> {
    let source_ticks = sample.time.div(&tick(base, bits)?, bits)?;
    let floor = source_ticks.round_integer(Rounding::Floor)?;
    if sample.side == SampleSide::Before
        && source_ticks.compare(&ExactRatio::integer(floor)).is_eq()
    {
        floor.checked_sub(1).ok_or(TimeError::Representation)
    } else {
        Ok(floor)
    }
}
pub(crate) fn min(a: RationalTime, b: RationalTime) -> RationalTime {
    if a.compare_time(b).is_le() { a } else { b }
}
pub(crate) fn max(a: RationalTime, b: RationalTime) -> RationalTime {
    if a.compare_time(b).is_ge() { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonunit_timebase_and_negative_boundary_seek_are_exact() {
        let base = TimeBase {
            numerator: 1001,
            denominator: 30000,
        };
        let at = timestamp(-3, base, 256).unwrap();
        assert_eq!(
            at.compare_time(RationalTime::new(-3003, 30000).unwrap()),
            std::cmp::Ordering::Equal
        );
        for (side, want) in [(SampleSide::After, -3), (SampleSide::Before, -4)] {
            assert_eq!(
                seek_tick(
                    &TimeSample {
                        time: ExactRatio::from_time(at),
                        side
                    },
                    base,
                    256
                )
                .unwrap(),
                want
            );
        }
        let between = TimeSample {
            time: ExactRatio::from_time(RationalTime::new(-3500, 30000).unwrap()),
            side: SampleSide::Before,
        };
        assert_eq!(seek_tick(&between, base, 256).unwrap(), -4);
    }
    #[test]
    fn timestamp_cancellation_reduces_before_wire_conversion_without_overflow() {
        let t = timestamp(
            i64::MAX,
            TimeBase {
                numerator: 1000,
                denominator: 1000,
            },
            256,
        )
        .unwrap();
        assert_eq!(t, RationalTime::new(i64::MAX, 1).unwrap());
        assert_eq!(
            timestamp(
                i64::MAX,
                TimeBase {
                    numerator: 2,
                    denominator: 1
                },
                256
            ),
            Err(TimeError::Representation)
        );
        assert_eq!(
            timestamp(
                1,
                TimeBase {
                    numerator: 1,
                    denominator: 0
                },
                256
            ),
            Err(TimeError::Denominator)
        );
    }
}
