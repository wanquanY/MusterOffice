use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;

/// Lossless host/resource size on JSON and WASM boundaries, including beyond 2^53.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ByteLength(u64);

#[derive(Debug, Error)]
#[error("expected a canonical unsigned 64-bit decimal byte length")]
pub struct ByteLengthError;

impl ByteLength {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}
impl TryFrom<String> for ByteLength {
    type Error = ByteLengthError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parsed = value.parse::<u64>().map_err(|_| ByteLengthError)?;
        if parsed.to_string() != value {
            return Err(ByteLengthError);
        }
        Ok(Self(parsed))
    }
}
impl From<ByteLength> for String {
    fn from(value: ByteLength) -> Self {
        value.0.to_string()
    }
}
impl JsonSchema for ByteLength {
    fn schema_name() -> Cow<'static, str> {
        "ByteLength".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string", "pattern":"^(0|[1-9][0-9]{0,19})$", "not":{"pattern":"[^0-9]"}, "x-integer-minimum":"0", "x-integer-maximum":"18446744073709551615", "description":"Canonical uint64 byte length. Range requires semantic validation."})
    }
}
