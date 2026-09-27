//! Caller-file adapter for the shared SDK. This is not a document store, job
//! service or authority model. The caller supplies every input/output handle.
mod inputs;
mod output;
pub mod workspace;
use mo_embedded_sdk::{
    NativeExporter,
    common::{ByteLength, Digest, RequestId},
    operation::{Failure, FailureCode, MAX_INVOCATION_BYTES, decode_invocation},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Native adapter resource acquisition/cleanup bound. Computation cancellation
/// stops acquisition promptly; cleanup retains its own bounded release window.
pub const SPOOL_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

pub struct FileCall<'a> {
    pub invocation: File,
    pub inputs: File,
    pub temporary_directory: &'a Path,
    pub output_directory: &'a Path,
    pub exporter: Option<&'a NativeExporter>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedComputation {
    pub request_id: RequestId,
    pub request_digest: Digest,
    pub result_file: PathBuf,
    pub result_byte_length: ByteLength,
    pub result_sha256: Digest,
    pub assets: usize,
    pub product_committed: bool,
}

/// A resolver belongs to the invoking host. File names in a manifest do not
/// grant access on their own. Copies are sealed and byte-verified before use.
pub fn compute(
    call: FileCall<'_>,
    resolve: &dyn Fn(&Path) -> std::io::Result<File>,
    cancelled: &dyn Fn() -> bool,
) -> Result<SavedComputation, Failure> {
    let result = (|| -> Result<SavedComputation, Error> {
        let bytes = read_limited(call.invocation, MAX_INVOCATION_BYTES, cancelled)?;
        let invocation = decode_invocation(std::str::from_utf8(&bytes)?)?;
        drop(bytes);
        let inputs = inputs::open(
            call.inputs,
            call.temporary_directory,
            &invocation.request,
            resolve,
            cancelled,
        )?;
        let borrowed = inputs.borrow()?;
        let execution = mo_embedded_sdk::execute(invocation, &borrowed, call.exporter, cancelled)?;
        drop(borrowed);
        inputs.discard()?;
        output::save(execution, call.output_directory, cancelled)
    })();
    result.map_err(|error| check(cancelled).err().unwrap_or_else(|| failure(error)))
}

fn check(cancelled: &dyn Fn() -> bool) -> Result<(), Failure> {
    if cancelled() {
        Err(Failure::new(
            FailureCode::Cancelled,
            "file computation cancelled",
        ))
    } else {
        Ok(())
    }
}

fn read_limited(
    mut input: File,
    limit: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, Error> {
    check(cancelled)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() {
        return Err("input must be a regular file".into());
    }
    if metadata.len() > limit as u64 {
        return Err(Failure::new(FailureCode::LimitExceeded, "input file byte limit").into());
    }
    let mut result = Vec::with_capacity(metadata.len() as usize);
    let mut buffer = [0; 65536];
    loop {
        check(cancelled)?;
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if result.len() + count > limit {
            return Err(Failure::new(FailureCode::LimitExceeded, "input file byte limit").into());
        }
        result.extend_from_slice(&buffer[..count]);
    }
    Ok(result)
}

fn failure(error: Error) -> Failure {
    use mo_embedded_sdk::{delivery::DeliveryError, operation::delivery_failure};
    let error = match error.downcast::<Failure>() {
        Ok(failure) => return *failure,
        Err(error) => error,
    };
    let error = match error.downcast::<DeliveryError>() {
        Ok(failure) => return delivery_failure(*failure),
        Err(error) => error,
    };
    let code = if error.is::<std::io::Error>() {
        FailureCode::IoFailure
    } else {
        FailureCode::InputInvalid
    };
    Failure::new(code, error.to_string())
}
