use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cmp::Ordering};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NumericError {
    #[error("expected a canonical signed 64-bit decimal string")]
    Integer,
    #[error("arithmetic overflow")]
    Overflow,
    #[error("timescale must be in 1..=1,000,000,000")]
    Timescale,
}

fn parse_integer(s: &str) -> Result<i64, NumericError> {
    let value = s.parse::<i64>().map_err(|_| NumericError::Integer)?;
    if value.to_string() != s {
        return Err(NumericError::Integer);
    }
    Ok(value)
}

macro_rules! signed_wire_integer {
    ($name:ident, $description:literal) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(i64);
        impl $name {
            pub const ZERO: Self = Self(0);
            pub const fn new(value: i64) -> Self { Self(value) }
            pub const fn get(self) -> i64 { self.0 }
            pub fn checked_add(self, rhs: Self) -> Result<Self, NumericError> {
                self.0.checked_add(rhs.0).map(Self).ok_or(NumericError::Overflow)
            }
            pub fn checked_sub(self, rhs: Self) -> Result<Self, NumericError> {
                self.0.checked_sub(rhs.0).map(Self).ok_or(NumericError::Overflow)
            }
        }
        impl TryFrom<String> for $name {
            type Error = NumericError;
            fn try_from(value: String) -> Result<Self, Self::Error> { parse_integer(&value).map(Self) }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self { value.0.to_string() }
        }
        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> { stringify!($name).into() }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({"type":"string", "pattern":"^(0|-?[1-9][0-9]{0,18})$", "description":$description, "not":{"pattern":"[^0-9-]"}, "x-integer-minimum":"-9223372036854775808", "x-integer-maximum":"9223372036854775807"})
            }
        }
    };
}

signed_wire_integer!(
    Emu,
    "Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation."
);
signed_wire_integer!(
    Ticks,
    "Signed int64 ticks. Range requires semantic validation."
);

impl Emu {
    pub const PER_POINT: i64 = 12_700;
    pub fn from_integer_points(points: i64) -> Result<Self, NumericError> {
        points
            .checked_mul(Self::PER_POINT)
            .map(Self)
            .ok_or(NumericError::Overflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Timescale(u32);

impl Timescale {
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl TryFrom<u32> for Timescale {
    type Error = NumericError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        (1..=1_000_000_000)
            .contains(&value)
            .then_some(Self(value))
            .ok_or(NumericError::Timescale)
    }
}
impl From<Timescale> for u32 {
    fn from(value: Timescale) -> Self {
        value.0
    }
}
impl JsonSchema for Timescale {
    fn schema_name() -> Cow<'static, str> {
        "Timescale".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"integer", "minimum":1, "maximum":1_000_000_000})
    }
}

/// Exact wire representation. Equality compares author values; compare_time compares instants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RationalTime {
    pub ticks: Ticks,
    pub timescale: Timescale,
}

impl RationalTime {
    pub fn new(ticks: i64, timescale: u32) -> Result<Self, NumericError> {
        Ok(Self {
            ticks: Ticks::new(ticks),
            timescale: timescale.try_into()?,
        })
    }
    pub fn compare_time(self, other: Self) -> Ordering {
        (i128::from(self.ticks.get()) * i128::from(other.timescale.get()))
            .cmp(&(i128::from(other.ticks.get()) * i128::from(self.timescale.get())))
    }
    pub fn normalized(self) -> Self {
        let mut a = self.ticks.get().unsigned_abs();
        let mut b = u64::from(self.timescale.get());
        while b != 0 {
            (a, b) = (b, a % b);
        }
        Self {
            ticks: Ticks::new((i128::from(self.ticks.get()) / i128::from(a)) as i64),
            timescale: Timescale((u64::from(self.timescale.get()) / a) as u32),
        }
    }
}
