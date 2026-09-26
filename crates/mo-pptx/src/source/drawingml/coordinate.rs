use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// ST_Coordinate retains its lexical precision and unit. Conversion is a
/// separate computation; source inspection must not round long decimal units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NativeCoordinate(String);
impl NativeCoordinate {
    pub fn lexical(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for NativeCoordinate {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let invalid = || "invalid native coordinate".to_owned();
        if let Some(number) = ["mm", "cm", "in", "pt", "pc", "pi"]
            .iter()
            .find_map(|unit| value.strip_suffix(unit))
        {
            let number = number.strip_prefix('-').unwrap_or(number);
            let (whole, fraction) = number
                .split_once('.')
                .map_or((number, None), |(a, b)| (a, Some(b)));
            if whole.is_empty()
                || !whole.bytes().all(|c| c.is_ascii_digit())
                || fraction.is_some_and(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()))
            {
                return Err(invalid());
            }
        } else {
            let digits = value.strip_prefix(['+', '-']).unwrap_or(&value);
            if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
                return Err(invalid());
            }
            let number: i64 = value.parse().map_err(|_| invalid())?;
            if !(-27_273_042_329_600..=27_273_042_316_900).contains(&number) {
                return Err(invalid());
            }
        }
        Ok(Self(value))
    }
}
impl From<NativeCoordinate> for String {
    fn from(value: NativeCoordinate) -> Self {
        value.0
    }
}
impl JsonSchema for NativeCoordinate {
    fn schema_name() -> Cow<'static, str> {
        "NativeCoordinate".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string", "pattern":"^([+-]?[0-9]+|-?[0-9]+(\\.[0-9]+)?(mm|cm|in|pt|pc|pi))$", "not":{"pattern":"[^0-9+.a-z\\-]"}, "description":"Native coordinate: bounded integer EMU or exact decimal universal measure."})
    }
}
