//! Operator-invoked binary channel. It shares the exact configured authority
//! and SQLite owner with stdio; it is never a model-controlled filesystem API.
use crate::config::{OperatorConfig, now};
use mo_common::ByteLength;
use mo_opc::ReaderAt;
use mo_operation_service::*;
use std::io::{self, Read, Write};

pub fn run(config: &OperatorConfig, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut host = config.host()?.connect()?;
    let context = config.context();
    let mut out = io::stdout().lock();
    match args {
        [command, id, offset] if command == "append" => {
            let mut bytes = Vec::new();
            io::stdin()
                .lock()
                .take((ASSET_CHUNK_BYTES + 1) as u64)
                .read_to_end(&mut bytes)?;
            let result = host.append_upload(
                &context,
                &UploadId::new(id)?,
                ByteLength::try_from(offset.clone())?,
                &bytes,
                now(),
            );
            let response = result
                .map(HostResponse::upload)
                .unwrap_or_else(|error| HostResponse::Failed { error, job: None });
            writeln!(out, "{}", serde_json::to_string(&response)?)?;
        }
        [command, id, offset, length] if command == "read-asset" => {
            let reader = host.open_asset(&context, &AssetId::new(id)?)?;
            let offset = ByteLength::try_from(offset.clone())?.get();
            let length = ByteLength::try_from(length.clone())?.get();
            if offset
                .checked_add(length)
                .is_none_or(|end| end > reader.info().descriptor.byte_length.get())
            {
                return Err("range exceeds authorized asset".into());
            }
            let mut bytes = vec![0; ASSET_CHUNK_BYTES];
            let mut copied = 0;
            while copied < length {
                let n = (length - copied).min(bytes.len() as u64) as usize;
                reader.read_exact_at(&mut bytes[..n], offset + copied)?;
                out.write_all(&bytes[..n])?;
                copied += n as u64;
            }
        }
        _ => {
            return Err(
                "expected append <upload-id> <offset> or read-asset <asset-id> <offset> <length>"
                    .into(),
            );
        }
    }
    out.flush()?;
    Ok(())
}
