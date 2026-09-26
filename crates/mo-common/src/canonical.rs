use crate::Digest;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CanonicalError {
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(
        "canonical JSON allows only exact JavaScript-safe integer numbers; use typed decimal strings otherwise"
    )]
    Number,
    #[error("canonical JSON depth exceeds 256")]
    Depth,
}

/// MusterOffice canonical JSON v1: UTF-8, lexicographic UTF-8 keys, no whitespace,
/// no Unicode normalization, safe integers only. This is not RFC 8785 JCS.
pub fn canonical_bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    value.serialize(crate::integer_json::IntegerJson::root())?;
    let value = serde_json::to_value(value)?;
    let mut output = Vec::new();
    write_value(&value, &mut output, 0)?;
    Ok(output)
}

fn write_value(value: &Value, out: &mut Vec<u8>, depth: usize) -> Result<(), CanonicalError> {
    if depth > 256 {
        return Err(CanonicalError::Depth);
    }
    match value {
        Value::Object(map) => {
            out.push(b'{');
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
            for (i, (key, val)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key)?;
                out.push(b':');
                write_value(val, out, depth + 1)?;
            }
            out.push(b'}');
        }
        Value::Array(array) => {
            out.push(b'[');
            for (i, val) in array.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(val, out, depth + 1)?;
            }
            out.push(b']');
        }
        Value::Number(number) => {
            const MAX_SAFE: i64 = 9_007_199_254_740_991;
            if !number
                .as_i64()
                .is_some_and(|n| (-MAX_SAFE..=MAX_SAFE).contains(&n))
            {
                return Err(CanonicalError::Number);
            }
            out.extend_from_slice(number.to_string().as_bytes());
        }
        _ => serde_json::to_writer(out, value)?,
    }
    Ok(())
}

/// Domain-separated SHA-256. This digest proves content identity, not authorization.
pub fn digest<T: Serialize + ?Sized>(domain: &str, value: &T) -> Result<Digest, CanonicalError> {
    let bytes = canonical_bytes(value)?;
    let mut hash = Sha256::new();
    hash.update((domain.len() as u64).to_be_bytes());
    hash.update(domain.as_bytes());
    hash.update(bytes);
    Ok(Digest::from_sha256(hash.finalize().into()))
}
