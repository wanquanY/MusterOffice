use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "identity must be 1..=128 ASCII characters, start with an alphanumeric, and contain only alphanumerics or _.:-"
)]
pub struct InvalidId;

fn valid_id(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
}

macro_rules! identity {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidId> {
                Self::try_from(value.into())
            }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidId;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                valid_id(&value).then_some(Self(value)).ok_or(InvalidId)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self { value.0 }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }
        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> { stringify!($name).into() }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({"type":"string", "pattern":"^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$", "maxLength":128, "not":{"pattern":"[^A-Za-z0-9_.:-]"}})
            }
        }
    )+};
}

identity!(
    DocumentId,
    SlideId,
    ObjectId,
    ParagraphId,
    RunId,
    ResourceId,
    FontId,
    ThemeId,
    MasterId,
    LayoutId,
    CellId,
    RowId,
    ColumnId,
    AxisId,
    SeriesId,
    DiagramPointId,
    MathNodeId,
    TimingNodeId,
    PlaybackSessionId,
    RequestId,
    OperationId,
    SectionId,
    TemplateParameterId,
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("digest must contain exactly 64 lowercase hexadecimal characters")]
pub struct InvalidDigest;

impl Digest {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Construct from the exact 32 output bytes of a SHA-256 computation.
    pub fn from_sha256(value: [u8; 32]) -> Self {
        let mut out = String::with_capacity(64);
        for byte in value {
            use std::fmt::Write;
            write!(&mut out, "{byte:02x}").expect("writing to a String is infallible");
        }
        Self(out)
    }
}
impl TryFrom<String> for Digest {
    type Error = InvalidDigest;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        (value.len() == 64
            && value
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)))
        .then_some(Self(value))
        .ok_or(InvalidDigest)
    }
}
impl From<Digest> for String {
    fn from(value: Digest) -> Self {
        value.0
    }
}
impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl JsonSchema for Digest {
    fn schema_name() -> Cow<'static, str> {
        "Digest".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string", "pattern":"^[0-9a-f]{64}$", "minLength":64, "maxLength":64, "not":{"pattern":"[^0-9a-f]"}})
    }
}
