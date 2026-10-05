use crate::ExactRatio;
use num_bigint::BigInt;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error, ser::SerializeStruct};
use std::borrow::Cow;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    numerator: String,
    denominator: String,
}

impl Serialize for ExactRatio {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let normalized = self.normalized();
        let mut value = serializer.serialize_struct("ExactRatio", 2)?;
        value.serialize_field("numerator", &normalized.numerator().to_string())?;
        value.serialize_field("denominator", &normalized.denominator().to_string())?;
        value.end()
    }
}
impl<'de> Deserialize<'de> for ExactRatio {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        fn integer<E: Error>(text: &str) -> Result<BigInt, E> {
            if text.is_empty() || text.len() > 1235 {
                return Err(E::custom("exact fraction exceeds its decimal budget"));
            }
            let value = text.parse::<BigInt>().map_err(E::custom)?;
            if value.to_string() != text {
                return Err(E::custom(
                    "exact fraction requires canonical decimal integers",
                ));
            }
            Ok(value)
        }
        ExactRatio::new(integer(&wire.numerator)?, integer(&wire.denominator)?, 4096)
            .map_err(D::Error::custom)
    }
}
impl JsonSchema for ExactRatio {
    fn schema_name() -> Cow<'static, str> {
        "ExactRatio".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"object","additionalProperties":false,"required":["numerator","denominator"],
            "description":"Exact computed fraction. Decimal strings are bounded to 4096 bits and canonicalized by the kernel; each computation can impose a smaller budget.",
            "properties":{"numerator":{"type":"string","minLength":1,"maxLength":1235,"pattern":"^(0|-?[1-9][0-9]*)$","not":{"pattern":"[^0-9-]"}},
            "denominator":{"type":"string","minLength":1,"maxLength":1234,"pattern":"^[1-9][0-9]*$","not":{"pattern":"[^0-9]"}}}})
    }
}
