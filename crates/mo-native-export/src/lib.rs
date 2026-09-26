//! Stateless native export computation for an existing host owner. No database,
//! job queue, credentials, Artifact commit or implicit font/resource discovery.
mod client;
mod storage;
mod wire;
mod worker;
pub use client::{NativeExportCandidate, NativeExporter};
use mo_common::Digest;
use mo_operation_service::{Failure, FailureCode};
use sha2::{Digest as _, Sha256};
use std::{io::Read, path::Path};
pub use worker::run_worker;

fn invalid(message: &'static str) -> Failure {
    Failure::new(FailureCode::InputInvalid, message)
}
fn storage_failure(_: std::io::Error) -> Failure {
    Failure::new(FailureCode::StorageFailure, "private export storage failed")
}

/// Host-configured executable bytes; no command or path comes from wire JSON.
pub fn executable_digest(path: &Path) -> Result<Digest, Failure> {
    let mut file = std::fs::File::open(path).map_err(storage_failure)?;
    let metadata = file.metadata().map_err(storage_failure)?;
    const MAX_EXECUTABLE: u64 = 128 * 1024 * 1024;
    if !metadata.is_file() || metadata.len() > MAX_EXECUTABLE {
        return Err(invalid("export worker executable size or type"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buffer).map_err(storage_failure)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_EXECUTABLE {
            return Err(invalid("export worker executable bytes"));
        }
        hash.update(&buffer[..n]);
    }
    Ok(Digest::from_sha256(hash.finalize().into()))
}
