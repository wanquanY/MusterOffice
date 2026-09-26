use crate::*;
use mo_common::{ByteLength, Digest, RequestId};
use mo_opc::{ReaderAt, ResultSink};
use sha2::{Digest as _, Sha256};
use std::io::Write;

#[derive(Clone, Copy)]
pub(crate) struct Spec<'a> {
    pub name: &'a str,
    pub mime: &'a str,
    pub role: AssetRole,
}

pub trait OutputStore {
    type Sink: ResultSink;
    /// Name is local to the candidate. The host must reserve/retain private
    /// storage and must not publish an asset when this function returns.
    fn create(
        &mut self,
        name: &str,
        media_type: &str,
        max_bytes: u64,
    ) -> Result<Self::Sink, DeliveryError>;
}
/// Actual sealed bytes matched the intended write digest and length. This type
/// cannot be deserialized into proof. Content-specific checks are separate.
pub struct ProducedArtifact<R> {
    pub(crate) name: String,
    pub(crate) asset: DeliveryAsset,
    pub(crate) reader: R,
}
impl<R> ProducedArtifact<R> {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn asset(&self) -> &DeliveryAsset {
        &self.asset
    }
    pub fn reader(&self) -> &R {
        &self.reader
    }
    pub fn into_parts(self) -> (String, DeliveryAsset, R) {
        (self.name, self.asset, self.reader)
    }
}
pub(crate) fn metadata(
    name: &str,
    mime: &str,
    role: AssetRole,
    digest: Digest,
    length: u64,
) -> Result<DeliveryAsset, DeliveryError> {
    let id = mo_common::digest(
        "musteroffice.delivery-asset/1",
        &(name, &digest, mime, role),
    )
    .map_err(|_| DeliveryError::Serialization)?;
    Ok(DeliveryAsset {
        id: RequestId::new(format!("asset:{id}")).map_err(|_| DeliveryError::Serialization)?,
        sha256: digest,
        byte_length: ByteLength::new(length),
        media_type: mime.into(),
        role,
    })
}
pub(crate) fn digest(
    reader: &dyn ReaderAt,
    length: u64,
    check: &dyn Fn() -> bool,
) -> Result<Digest, DeliveryError> {
    let mut hash = Sha256::new();
    let mut offset = 0;
    let mut buffer = [0; 65536];
    while offset < length {
        cancel(check)?;
        let n = (length - offset).min(buffer.len() as u64) as usize;
        reader.read_exact_at(&mut buffer[..n], offset)?;
        hash.update(&buffer[..n]);
        offset += n as u64;
    }
    cancel(check)?;
    Ok(Digest::from_sha256(hash.finalize().into()))
}
pub(crate) fn seal<S: ResultSink>(
    mut sink: S,
    spec: Spec<'_>,
    expected: &Digest,
    length: u64,
    check: &dyn Fn() -> bool,
) -> Result<ProducedArtifact<S::Reader>, DeliveryError> {
    cancel(check)?;
    sink.flush()?;
    cancel(check)?;
    let sealed = sink.seal()?;
    if sealed.byte_length != length || &digest(&sealed.reader, length, check)? != expected {
        return Err(DeliveryError::Invalid(
            "stored output differs from intended bytes",
        ));
    }
    let asset = metadata(spec.name, spec.mime, spec.role, expected.clone(), length)?;
    Ok(ProducedArtifact {
        name: spec.name.into(),
        asset,
        reader: sealed.reader,
    })
}
pub(crate) fn copy<S: OutputStore>(
    store: &mut S,
    spec: Spec<'_>,
    content: Content<'_>,
    expected: Option<&Digest>,
    limit: u64,
    check: &dyn Fn() -> bool,
) -> Result<ProducedArtifact<<S::Sink as ResultSink>::Reader>, DeliveryError> {
    if content.byte_length > limit {
        return Err(DeliveryError::Limit("artifact bytes"));
    }
    cancel(check)?;
    let mut sink = store.create(spec.name, spec.mime, content.byte_length)?;
    let mut hash = Sha256::new();
    let mut offset = 0;
    let mut buffer = [0; 65536];
    while offset < content.byte_length {
        cancel(check)?;
        let n = (content.byte_length - offset).min(buffer.len() as u64) as usize;
        content.reader.read_exact_at(&mut buffer[..n], offset)?;
        sink.write_all(&buffer[..n])?;
        hash.update(&buffer[..n]);
        offset += n as u64;
    }
    let computed = Digest::from_sha256(hash.finalize().into());
    if expected.is_some_and(|e| e != &computed) {
        return Err(DeliveryError::Invalid("resource digest conflict"));
    }
    seal(sink, spec, &computed, offset, check)
}
