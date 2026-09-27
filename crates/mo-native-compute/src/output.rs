use mo_embedded_sdk::{
    Execution,
    common::{Digest, RequestId},
    delivery::{self, AssetRole, Content, DeliveryError, DeliverySource},
    opc::ReaderAt,
};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

/// This one explicitly requested output directory belongs to the caller.
/// No library, history, background task, or output lookup service is created.
struct Directory {
    path: PathBuf,
    retained: bool,
}
impl Drop for Directory {
    fn drop(&mut self) {
        if !self.retained {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
struct Stored(BTreeMap<RequestId, (u64, rawzip::FileReader)>);
impl DeliverySource for Stored {
    fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError> {
        let (length, reader) = self
            .0
            .get(id)
            .ok_or(DeliveryError::Invalid("output missing"))?;
        Ok(Content {
            reader,
            byte_length: *length,
        })
    }
}
fn create(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
struct JsonWriter<'a> {
    cancelled: &'a dyn Fn() -> bool,
    file: File,
    bytes: u64,
    hash: Sha256,
}
impl Write for JsonWriter<'_> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        crate::check(self.cancelled).map_err(std::io::Error::other)?;
        if self
            .bytes
            .checked_add(data.len() as u64)
            .is_none_or(|n| n > 66 * 1024 * 1024)
        {
            return Err(std::io::Error::other("response JSON limit"));
        }
        let n = self.file.write(data)?;
        self.hash.update(&data[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
fn json(
    path: &Path,
    value: &impl serde::Serialize,
    cancelled: &dyn Fn() -> bool,
) -> Result<(u64, Digest), crate::Error> {
    let mut output = std::io::BufWriter::with_capacity(
        65536,
        JsonWriter {
            cancelled,
            file: create(path)?,
            bytes: 0,
            hash: Sha256::new(),
        },
    );
    serde_json::to_writer(&mut output, value)?;
    output.flush()?;
    let writer = output.into_inner().map_err(|error| error.into_error())?;
    writer.file.sync_all()?;
    if writer.file.metadata()?.len() != writer.bytes {
        return Err("saved response length differs".into());
    }
    let expected = Digest::from_sha256(writer.hash.finalize().into());
    let reader: rawzip::FileReader = writer.file.into();
    let actual = hash(&reader, writer.bytes, cancelled)?;
    if actual != expected {
        return Err("saved response bytes differ".into());
    }
    Ok((writer.bytes, expected))
}
fn hash(
    reader: &dyn ReaderAt,
    length: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<Digest, crate::Error> {
    let mut hash = Sha256::new();
    let mut offset = 0;
    let mut chunk = [0; 65536];
    while offset < length {
        crate::check(cancelled)?;
        let n = (length - offset).min(chunk.len() as u64) as usize;
        reader.read_exact_at(&mut chunk[..n], offset)?;
        hash.update(&chunk[..n]);
        offset += n as u64;
    }
    Ok(Digest::from_sha256(hash.finalize().into()))
}
pub(super) fn save(
    execution: Execution,
    destination: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<crate::SavedComputation, crate::Error> {
    crate::check(cancelled)?;
    // Exclusive creation rejects existing caller data before any output write.
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let mut builder = builder;
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(destination)?;
    let mut directory = Directory {
        path: destination.into(),
        retained: false,
    };
    let mut stored = Stored(BTreeMap::new());
    let mut files = Vec::new();
    if let Some(candidate) = execution.export() {
        for (i, asset) in candidate.assets().iter().enumerate() {
            let extension = match asset.role {
                AssetRole::Pptx => "pptx",
                AssetRole::Preview => "png",
                _ => "bin",
            };
            let name = format!("asset-{i:04}.{extension}");
            let content = candidate.open(&asset.id)?;
            let mut file = create(&directory.path.join(&name))?;
            let mut offset = 0;
            let mut buffer = [0; 65536];
            while offset < content.byte_length {
                crate::check(cancelled)?;
                let n = (content.byte_length - offset).min(buffer.len() as u64) as usize;
                content.reader.read_exact_at(&mut buffer[..n], offset)?;
                file.write_all(&buffer[..n])?;
                offset += n as u64;
            }
            file.sync_all()?;
            if file.metadata()?.len() != asset.byte_length.get() {
                return Err("saved output length differs".into());
            }
            stored
                .0
                .insert(asset.id.clone(), (asset.byte_length.get(), file.into()));
            files.push(serde_json::json!({ "file": name, "asset": asset }));
        }
        let inspected = delivery::inspect(
            &candidate.receipt().bundle,
            candidate.expectation(),
            &stored,
            Default::default(),
            cancelled,
        )?;
        json(
            &directory.path.join("inspection.json"),
            inspected.report(),
            cancelled,
        )?;
    }
    json(&directory.path.join("files.json"), &files, cancelled)?;
    let (length, digest) = json(
        &directory.path.join("result.json"),
        execution.receipt(),
        cancelled,
    )?;
    let response = crate::SavedComputation {
        request_id: execution.receipt().request_id.clone(),
        request_digest: execution.receipt().request_digest.clone(),
        result_file: directory.path.join("result.json"),
        result_byte_length: mo_embedded_sdk::common::ByteLength::new(length),
        result_sha256: digest,
        assets: files.len(),
        product_committed: false,
    };
    // Cleanup is part of completion; do not report success if retained private
    // worker files cannot be released. Caller files remain under this guard.
    let (_, candidate) = execution.into_parts();
    if let Some(candidate) = candidate {
        candidate.discard_with_wait(crate::SPOOL_WAIT)?;
    }
    drop(stored);
    crate::check(cancelled)?;
    directory.retained = true;
    Ok(response)
}
