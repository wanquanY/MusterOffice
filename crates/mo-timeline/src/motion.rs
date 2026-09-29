//! Exact slide-relative coordinates. No binary float or implicit source rounding.
use crate::{TimelineError, exact::Ratio};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A decimal fraction of the slide width/height. Bounded to nine integer and
/// eighteen fractional digits; values outside this computation profile fail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MotionCoordinate(String);
impl MotionCoordinate {
    pub fn lexical(&self) -> &str {
        &self.0
    }
    pub(crate) fn parts(&self) -> (i128, u64) {
        let digits = self.0.split_once('.').map_or(0, |(_, f)| f.len());
        (
            self.0.replace('.', "").parse().expect("validated decimal"),
            10u64.pow(digits as u32),
        )
    }
    pub(crate) fn ratio(&self, bits: u64) -> Result<Ratio, TimelineError> {
        let (n, d) = self.parts();
        Ratio::new(n.into(), d.into(), bits)
    }
    /// Normalize a native relative line endpoint without losing any digit.
    pub fn checked_add(&self, other: &Self) -> Result<Self, String> {
        const UNIT: i128 = 1_000_000_000_000_000_000;
        let scaled = |v: &Self| {
            let (n, d) = v.parts();
            n * (UNIT / i128::from(d))
        };
        let sum = scaled(self) + scaled(other);
        let magnitude = sum.abs();
        let sign = if sum < 0 { "-" } else { "" };
        format!("{sign}{}.{:018}", magnitude / UNIT, magnitude % UNIT).try_into()
    }
}
impl TryFrom<String> for MotionCoordinate {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let fail =
            || "motion coordinate requires a bounded decimal with a leading integer".to_owned();
        if value.len() > 29 {
            return Err(fail());
        }
        let unsigned = value.strip_prefix('-').unwrap_or(&value);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if whole.is_empty()
            || whole.len() > 9
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || (whole.len() > 1 && whole.starts_with('0'))
            || fraction.len() > 18
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || (unsigned.contains('.') && fraction.is_empty())
        {
            return Err(fail());
        }
        let fraction = fraction.trim_end_matches('0');
        let negative = value.starts_with('-') && (whole != "0" || !fraction.is_empty());
        Ok(Self(format!(
            "{}{}{}{}",
            if negative { "-" } else { "" },
            whole,
            if fraction.is_empty() { "" } else { "." },
            fraction
        )))
    }
}
impl From<MotionCoordinate> for String {
    fn from(value: MotionCoordinate) -> Self {
        value.0
    }
}
impl JsonSchema for MotionCoordinate {
    fn schema_name() -> Cow<'static, str> {
        "MotionCoordinate".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string", "maxLength":29,
            "pattern":"^-?(0|[1-9][0-9]{0,8})(\\.[0-9]{1,18})?$",
            "not":{"pattern":"[^0-9.\\-]"},
            "description":"Exact decimal fraction of the slide dimension; canonicalized without rounding."})
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionPoint {
    pub x: MotionCoordinate,
    pub y: MotionCoordinate,
}

/// Connected native path. Coordinates are absolute offsets from the original
/// layout center, measured in slide fractions. Pacing uses length in this
/// normalized coordinate space, before scaling the axes to slide dimensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionPath {
    pub from: MotionPoint,
    pub segments: Vec<MotionSegment>,
}

/// Source control points remain editable; subdivision belongs only to the
/// immutable playback plan. Close returns to the initial `from` point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum MotionSegment {
    Line {
        to: MotionPoint,
    },
    Cubic {
        control1: MotionPoint,
        control2: MotionPoint,
        to: MotionPoint,
    },
    Close,
}
