use crate::{PcmError, Result, bytes, check};
use mo_common::Digest;
use sha2::{Digest as _, Sha256};

/// One canonical encoding for known-extent and EOF-sealed streams. All callers
/// supply samples already validated by PcmBlock; no host byte order leaks out.
pub(crate) fn encode(samples: &[f32], cancelled: &dyn Fn() -> bool) -> Result<(Vec<u8>, Digest)> {
    let mut data = bytes(samples.len().checked_mul(4).ok_or(PcmError::Budget)?)?;
    let mut hash = Sha256::new();
    for chunk in samples.chunks(4096) {
        check(cancelled)?;
        let begin = data.len();
        for value in chunk {
            data.extend_from_slice(&value.to_le_bytes());
        }
        hash.update(&data[begin..]);
    }
    check(cancelled)?;
    Ok((data, Digest::from_sha256(hash.finalize().into())))
}
