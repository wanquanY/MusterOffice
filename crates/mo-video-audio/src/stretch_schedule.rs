use crate::{AudioError, Result, cancel};
use mo_common::Ticks;
use mo_time::{ExactRatio as R, Rounding};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StretchAnchor {
    /// Positions are project-rate samples and may be fractional.
    pub output: R,
    pub input: R,
}
#[derive(Debug, Clone, Serialize)]
pub struct StretchSchedule {
    anchors: Vec<StretchAnchor>,
    #[serde(skip)]
    rates: Vec<R>,
    #[serde(skip)]
    boundaries: Vec<u64>,
    output_frames: Ticks,
    source_frames: Ticks,
    #[serde(skip)]
    bits: u64,
}
impl StretchSchedule {
    pub fn new(
        mut anchors: Vec<StretchAnchor>,
        bits: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        if !(128..=4096).contains(&bits) || !(2..=100_000).contains(&anchors.len()) {
            return Err(AudioError::Invalid("stretch schedule limits"));
        }
        let zero = R::integer(0);
        if anchors[0].input.compare(&zero).is_ne() || anchors[0].output.compare(&zero).is_ne() {
            return Err(AudioError::Invalid("stretch schedule origin"));
        }
        let mut rates = Vec::new();
        let mut boundaries = Vec::new();
        for (i, pair) in anchors.windows(2).enumerate() {
            cancel(check)?;
            if pair[0].input.compare(&pair[1].input).is_ge()
                || pair[0].output.compare(&pair[1].output).is_ge()
            {
                return Err(AudioError::Invalid("stretch schedule must advance"));
            }
            let rate = pair[1]
                .input
                .sub(&pair[0].input, bits)?
                .div(&pair[1].output.sub(&pair[0].output, bits)?, bits)?;
            if rate.compare(&R::integer(64)).is_gt()
                || rate
                    .compare(&R::integer(1).div(&R::integer(64), bits)?)
                    .is_lt()
            {
                return Err(AudioError::Unsupported("stretch rate outside 1/64..64"));
            }
            rates.push(rate);
            if i + 2 < anchors.len() {
                boundaries.push(pair[1].output.round_integer(Rounding::Ceil)? as u64);
            }
        }
        let last = anchors.last().expect("two anchors");
        let output_frames = Ticks::new(last.output.round_integer(Rounding::Ceil)?);
        let source_frames = Ticks::new(last.input.round_integer(Rounding::Ceil)?);
        for a in &mut anchors {
            cancel(check)?;
            a.input = a.input.normalized();
            a.output = a.output.normalized();
        }
        Ok(Self {
            anchors,
            rates,
            boundaries,
            output_frames,
            source_frames,
            bits,
        })
    }
    pub fn constant(source_frames: u64, output_frames: u64) -> Result<Self> {
        let source = i64::try_from(source_frames).map_err(|_| AudioError::Budget)?;
        let output = i64::try_from(output_frames).map_err(|_| AudioError::Budget)?;
        Self::new(
            vec![
                StretchAnchor {
                    output: R::integer(0),
                    input: R::integer(0),
                },
                StretchAnchor {
                    output: R::integer(output),
                    input: R::integer(source),
                },
            ],
            128,
            &|| false,
        )
    }
    pub fn output_frames(&self) -> u64 {
        self.output_frames.get() as u64
    }
    pub fn source_frames(&self) -> u64 {
        self.source_frames.get() as u64
    }
    pub fn anchors(&self) -> &[StretchAnchor] {
        &self.anchors
    }
    pub(crate) fn initial_rate_hint(&self) -> Result<f64> {
        let rate = &self.rates[0];
        let q = u64::try_from((rate.numerator() << 32) / rate.denominator())
            .map_err(|_| AudioError::Budget)?;
        Ok(q as f64 / 4294967296.)
    }
    pub(crate) fn input_position(&self, output: u64) -> Result<u64> {
        let output = R::new(output.into(), 1.into(), self.bits)?;
        let right = self
            .anchors
            .partition_point(|a| a.output.compare(&output).is_le());
        let index = right.saturating_sub(1).min(self.rates.len() - 1);
        let anchor = &self.anchors[index];
        let position = anchor.input.add(
            &output
                .sub(&anchor.output, self.bits)?
                .mul(&self.rates[index], self.bits)?,
            self.bits,
        )?;
        u64::try_from(position.numerator() / position.denominator()).map_err(|_| AudioError::Budget)
    }
    pub(crate) fn next_boundary(&self, output: u64, proposed: u64) -> u64 {
        let index = self.boundaries.partition_point(|b| *b <= output);
        self.boundaries
            .get(index)
            .copied()
            .unwrap_or(proposed)
            .min(proposed)
    }
    pub(crate) fn preroll_calls(&self, delay: u32) -> usize {
        let mut position = 0;
        let mut calls = 0;
        while position <= u64::from(delay) {
            position = self.next_boundary(position, position + u64::from(crate::STRETCH_QUANTUM));
            calls += 1;
        }
        calls
    }
}
