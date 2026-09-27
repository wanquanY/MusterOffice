use mo_embedded_sdk::{
    Inputs,
    common::Digest,
    delivery::DeliveryLimits,
    native_io::{ExecutionSpool, ExecutionSpoolRoot, FileSpool, SealedFile},
    opc::ResultSink,
    operation::{AssetId, AssetInfo, DocumentAction, OperationRequest},
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Local file bindings, outside the portable operation contract. The caller
/// must resolve each name through its own explicitly supplied file bridge.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileInput {
    info: AssetInfo,
    file: PathBuf,
}
pub(super) struct InputFiles {
    files: Vec<(AssetInfo, SealedFile)>,
    // Readers close before the execution-private directory is removed.
    _workspace: Option<ExecutionSpool>,
}
impl InputFiles {
    pub fn borrow(&self) -> Result<Inputs<'_>, crate::Error> {
        let mut inputs = Inputs::new();
        for (info, reader) in &self.files {
            inputs.insert_reader(info.clone(), reader)?;
        }
        Ok(inputs)
    }
    pub fn discard(mut self) -> Result<(), crate::Error> {
        self.release()
    }
    fn release(&mut self) -> Result<(), crate::Error> {
        let mut first_error = None;
        for (_, reader) in self.files.drain(..) {
            if let Err(error) = reader.discard() {
                first_error.get_or_insert(crate::Error::from(error));
            }
        }
        if let Some(workspace) = self._workspace.take()
            && let Err(error) = workspace.discard_with_wait(crate::SPOOL_WAIT)
        {
            first_error.get_or_insert(crate::Error::from(error));
        }
        first_error.map_or(Ok(()), Err)
    }
}
impl Drop for InputFiles {
    fn drop(&mut self) {
        let _ = self.release();
    }
}
pub(super) fn open(
    manifest: File,
    spool: &Path,
    request: &OperationRequest,
    resolve: &dyn Fn(&Path) -> std::io::Result<File>,
    cancelled: &dyn Fn() -> bool,
) -> Result<InputFiles, crate::Error> {
    let raw = crate::read_limited(manifest, 1024 * 1024, cancelled)?;
    let bindings: Vec<FileInput> =
        mo_embedded_sdk::common::from_json_str(std::str::from_utf8(&raw)?)?;
    drop(raw);
    let required: BTreeSet<&AssetId> = match &request.action {
        DocumentAction::Import { source, .. } => [&source.asset_id].into_iter().collect(),
        DocumentAction::Export { settings, .. } => settings
            .resources
            .iter()
            .map(|r| &r.asset_id)
            .chain(settings.font_asset_id.iter())
            .collect(),
        _ => BTreeSet::new(),
    };
    let limits = DeliveryLimits::default();
    if bindings.len() > limits.max_artifacts {
        return Err("input count limit".into());
    }
    let mut supplied = BTreeSet::new();
    let mut total = 0u64;
    for input in &bindings {
        let n = input.info.descriptor.byte_length.get();
        if !supplied.insert(&input.info.id)
            || n > limits.max_asset_bytes
            || input.info.descriptor.media_type.is_empty()
            || input.info.descriptor.media_type.len() > 255
            || input.file.as_os_str().is_empty()
        {
            return Err("invalid input binding or limit".into());
        }
        total = total
            .checked_add(n)
            .filter(|n| *n <= limits.max_total_bytes)
            .ok_or("input aggregate limit")?;
        if let DocumentAction::Export { settings, .. } = &request.action
            && settings.font_asset_id.as_ref() == Some(&input.info.id)
            && n > limits.max_font_bytes
        {
            return Err("font input limit".into());
        }
    }
    if supplied != required {
        return Err("input bindings differ from required resources".into());
    }
    let mut result = InputFiles {
        files: Vec::new(),
        _workspace: None,
    };
    if bindings.is_empty() {
        return Ok(result);
    }
    let workspace =
        ExecutionSpoolRoot::open(spool)?.create_with_wait(crate::SPOOL_WAIT, cancelled)?;
    result._workspace = Some(workspace);
    let directory = result
        ._workspace
        .as_ref()
        .expect("execution workspace")
        .path();
    for binding in bindings {
        crate::check(cancelled)?;
        let mut source = resolve(&binding.file)?;
        if !source.metadata()?.is_file()
            || source.metadata()?.len() != binding.info.descriptor.byte_length.get()
        {
            return Err("input must be a regular file of the declared length".into());
        }
        let length = binding.info.descriptor.byte_length.get();
        let mut sink = FileSpool::create(directory, length)?;
        let mut hash = Sha256::new();
        let mut remaining = length;
        let mut chunk = [0; 65536];
        while remaining != 0 {
            crate::check(cancelled)?;
            let n = remaining.min(chunk.len() as u64) as usize;
            source.read_exact(&mut chunk[..n])?;
            hash.update(&chunk[..n]);
            sink.write_all(&chunk[..n])?;
            remaining -= n as u64;
        }
        if source.read(&mut chunk[..1])? != 0
            || Digest::from_sha256(hash.finalize().into()) != binding.info.descriptor.sha256
        {
            return Err("actual input bytes differ from the declared identity".into());
        }
        let sealed = sink.seal()?;
        if sealed.byte_length != length {
            return Err("sealed input length differs".into());
        }
        result.files.push((binding.info, sealed.reader));
    }
    Ok(result)
}
