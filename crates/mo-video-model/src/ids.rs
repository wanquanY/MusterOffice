use mo_common::{InvalidId, ObjectId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Distinct namespaces share the existing validated wire identity, not its ownership.
macro_rules! video_ids {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        #[schemars(transparent)]
        pub struct $name(ObjectId);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidId> { ObjectId::new(value).map(Self) }
            pub fn as_str(&self) -> &str { self.0.as_str() }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
        }
    )+};
}
video_ids!(
    SequenceId,
    TrackId,
    ClipId,
    StreamId,
    LinkGroupId,
    TransitionId
);
