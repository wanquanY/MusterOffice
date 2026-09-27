use super::bridge::{check, io_failure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use mo_embedded_sdk::operation::{Failure, FailureCode};
use mo_native_compute::workspace::{Directory, leaf};
use rmcp::model::ResourceContents;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};

pub const CHUNK_BYTES: u64 = 256 * 1024;
pub const PREFIX: &str = "musteroffice://output/";

fn invalid() -> Failure {
    Failure::new(
        FailureCode::InputInvalid,
        "expected output URI with portable directory/file and optional ?offset=<u64>&length=<1..262144>",
    )
}
fn decimal(s: &str) -> Result<u64, Failure> {
    let n = s.parse::<u64>().map_err(|_| invalid())?;
    if n.to_string() != s {
        return Err(invalid());
    }
    Ok(n)
}
pub(super) fn read(
    root: &Directory,
    uri: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<ResourceContents, Failure> {
    check(cancelled)?;
    if uri.len() > 400 {
        return Err(invalid());
    }
    let value = uri.strip_prefix(PREFIX).ok_or_else(invalid)?;
    let (path, range) = value
        .split_once('?')
        .map_or((value, None), |(p, q)| (p, Some(q)));
    let (directory, name) = path.split_once('/').ok_or_else(invalid)?;
    leaf(directory).map_err(|_| invalid())?;
    leaf(name).map_err(|_| invalid())?;
    let (offset, length) = if let Some(range) = range {
        let (offset, length) = range.split_once('&').ok_or_else(invalid)?;
        let offset = decimal(offset.strip_prefix("offset=").ok_or_else(invalid)?)?;
        let length = decimal(length.strip_prefix("length=").ok_or_else(invalid)?)?;
        if length == 0 || length > CHUNK_BYTES {
            return Err(invalid());
        }
        (offset, Some(length))
    } else {
        (0, None)
    };
    let mut input = root
        .directory(directory)
        .map_err(io_failure)?
        .read(name)
        .map_err(io_failure)?;
    let total = input.metadata().map_err(io_failure)?.len();
    if offset > total {
        return Err(invalid());
    }
    let length = length.unwrap_or(total);
    if length > CHUNK_BYTES {
        return Err(Failure::new(
            FailureCode::LimitExceeded,
            "resource exceeds inline bytes; use bounded range URI or the caller's native file channel",
        ));
    }
    let count = length.min(total - offset) as usize;
    input.seek(SeekFrom::Start(offset)).map_err(io_failure)?;
    let mut bytes = vec![0; count];
    for chunk in bytes.chunks_mut(65536) {
        check(cancelled)?;
        input.read_exact(chunk).map_err(io_failure)?;
    }
    check(cancelled)?;
    if input.metadata().map_err(io_failure)?.len() != total {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "output file length changed during read",
        ));
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let content = if range.is_none() && name.ends_with(".json") {
        ResourceContents::text(String::from_utf8(bytes).map_err(|_| invalid())?, uri)
            .with_mime_type("application/json")
    } else {
        // Ranges are bytes of a document, not complete images/text fragments.
        ResourceContents::blob(STANDARD.encode(bytes), uri)
            .with_mime_type("application/octet-stream")
    };
    Ok(content.with_meta(rmcp::model::MetaObject(serde_json::json!({"io.musteroffice/range":{"offset":offset.to_string(),"byteLength":count.to_string(),"totalByteLength":total.to_string(),"sha256":digest}}).as_object().unwrap().clone())))
}
