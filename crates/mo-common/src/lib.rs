//! Platform-independent wire primitives. This crate performs no host I/O.

mod byte_length;
mod canonical;
mod ids;
mod integer_json;
mod strict_json;
mod units;

pub use byte_length::{ByteLength, ByteLengthError};
pub use canonical::{CanonicalError, canonical_bytes, digest};
pub use ids::*;
pub use strict_json::from_json_str;
pub use units::*;

/// Stable runtime schema identity shared by on-demand discovery and file
/// generation. The caller supplies a registered schema name, never a path.
pub fn runtime_schema(name: &str, mut schema: schemars::Schema) -> schemars::Schema {
    schema.insert(
        "$id".into(),
        format!("urn:musteroffice:runtime:0.1-draft:{name}").into(),
    );
    schema
}
