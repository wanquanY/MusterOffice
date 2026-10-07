use crate::{ExactRatio, TimeError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A one-sided sample at an exact instant. Reversal flips the side rather than inventing an epsilon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SampleSide {
    After,
    Before,
}
impl SampleSide {
    pub fn reversed(self) -> Self {
        match self {
            Self::After => Self::Before,
            Self::Before => Self::After,
        }
    }
}

/// Exact presentation interval; endpoints need not fit a device or document time base.
/// After samples use [start,end), Before samples use (start,end]. Validate at admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactInterval {
    pub start: ExactRatio,
    pub end: ExactRatio,
}
impl ExactInterval {
    pub fn contains(&self, time: &ExactRatio, side: SampleSide) -> bool {
        match side {
            SampleSide::After => {
                self.start.compare(time).is_le() && time.compare(&self.end).is_lt()
            }
            SampleSide::Before => {
                self.start.compare(time).is_lt() && time.compare(&self.end).is_le()
            }
        }
    }
    pub fn validate(&self, bits: u64) -> Result<(), TimeError> {
        self.start.check_budget(bits)?;
        self.end.check_budget(bits)?;
        if self.start.compare(&self.end).is_ge() {
            return Err(TimeError::Interval);
        }
        Ok(())
    }
}
