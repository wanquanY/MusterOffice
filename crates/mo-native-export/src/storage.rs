use crate::*;
use mo_common::RequestId;
use mo_native_io::{FileSpool, SealedFile};
use mo_opc::{ReaderAt, ResultSink};
use mo_operation_service::{AssetId, AssetInfo, ExportAsset, ExportAssets};
use mo_presentation_delivery::{
    Content, DeliveryAsset, DeliveryError, DeliveryLimits, DeliverySource, OutputStore,
};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) struct Inputs(pub BTreeMap<AssetId, (AssetInfo, SealedFile)>);
impl ExportAssets for Inputs {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure> {
        let (info, reader) = self
            .0
            .get(id)
            .ok_or_else(|| invalid("missing input asset"))?;
        Ok(ExportAsset { info, reader })
    }
}
pub(crate) struct Outputs {
    pub root: PathBuf,
}
impl OutputStore for Outputs {
    type Sink = FileSpool;
    fn create(&mut self, _: &str, _: &str, limit: u64) -> Result<FileSpool, DeliveryError> {
        Ok(FileSpool::create(&self.root, limit)?)
    }
}
pub(crate) struct ReceivedAssets(pub BTreeMap<RequestId, (DeliveryAsset, SealedFile)>);
impl DeliverySource for ReceivedAssets {
    fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError> {
        let (_, reader) = self
            .0
            .get(id)
            .ok_or(DeliveryError::Invalid("received asset missing"))?;
        Ok(Content {
            reader,
            byte_length: reader.byte_length(),
        })
    }
}
pub(crate) fn receive(
    reader: &mut impl Read,
    root: &Path,
    length: u64,
) -> Result<SealedFile, Failure> {
    let mut sink = FileSpool::create(root, length).map_err(storage_failure)?;
    let copied = std::io::copy(&mut reader.take(length), &mut sink).map_err(storage_failure)?;
    if copied != length {
        return Err(invalid("truncated export resource"));
    }
    let sealed = sink.seal().map_err(storage_failure)?;
    if sealed.byte_length != length {
        return Err(invalid("sealed export resource length"));
    }
    Ok(sealed.reader)
}
pub(crate) fn stream(
    reader: &dyn ReaderAt,
    length: u64,
    output: &mut impl Write,
) -> Result<(), Failure> {
    let mut offset = 0;
    let mut buffer = [0; 65536];
    while offset < length {
        let n = (length - offset).min(buffer.len() as u64) as usize;
        reader
            .read_exact_at(&mut buffer[..n], offset)
            .map_err(storage_failure)?;
        output.write_all(&buffer[..n]).map_err(storage_failure)?;
        offset += n as u64;
    }
    Ok(())
}
pub(crate) fn verify(reader: &dyn ReaderAt, length: u64, expected: &Digest) -> Result<(), Failure> {
    use sha2::Digest as _;
    let mut hash = sha2::Sha256::new();
    let mut offset = 0;
    let mut buffer = [0; 65536];
    while offset < length {
        let n = (length - offset).min(buffer.len() as u64) as usize;
        reader
            .read_exact_at(&mut buffer[..n], offset)
            .map_err(storage_failure)?;
        hash.update(&buffer[..n]);
        offset += n as u64;
    }
    if &Digest::from_sha256(hash.finalize().into()) != expected {
        return Err(Failure::new(
            FailureCode::ResourceConflict,
            "actual input resource digest differs",
        ));
    }
    Ok(())
}
pub(crate) fn output_preflight(assets: &[DeliveryAsset]) -> Result<(), Failure> {
    let limits = DeliveryLimits::default();
    if assets.is_empty() || assets.len() > limits.max_artifacts {
        return Err(invalid("output asset count"));
    }
    let mut total = 0u64;
    let mut ids = std::collections::BTreeSet::new();
    for asset in assets {
        let n = asset.byte_length.get();
        if n > limits.max_asset_bytes || !ids.insert(&asset.id) {
            return Err(invalid("output asset limit or identity"));
        }
        total = total
            .checked_add(n)
            .filter(|n| *n <= limits.max_total_bytes)
            .ok_or_else(|| invalid("output total bytes"))?;
    }
    Ok(())
}
