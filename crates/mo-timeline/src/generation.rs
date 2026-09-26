use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Host-issued generation; distinct from resource byte lengths and event cursors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PlaybackGeneration(u64);
impl PlaybackGeneration {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
impl TryFrom<String> for PlaybackGeneration {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let n = value
            .parse::<u64>()
            .map_err(|_| "expected canonical uint64 generation")?;
        if n.to_string() != value {
            return Err("expected canonical uint64 generation");
        }
        Ok(Self(n))
    }
}
impl From<PlaybackGeneration> for String {
    fn from(v: PlaybackGeneration) -> Self {
        v.0.to_string()
    }
}
impl JsonSchema for PlaybackGeneration {
    fn schema_name() -> Cow<'static, str> {
        "PlaybackGeneration".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","pattern":"^(0|[1-9][0-9]{0,19})$","not":{"pattern":"[^0-9]"},"x-integer-maximum":"18446744073709551615","description":"Canonical uint64 playback generation; never wraps."})
    }
}
