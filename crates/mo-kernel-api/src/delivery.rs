//! Portable bounded bridge for transport-independent delivery admission.
use mo_common::{ByteLength, RequestId};
use mo_opc::ReaderAt;
use mo_presentation_delivery::{
    Content, DeliveryBundle, DeliveryError, DeliveryExpectation, DeliveryLimits, DeliverySource,
    ReceiptInspection,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryInspectRequest {
    pub bundle: DeliveryBundle,
    pub expected: DeliveryExpectation,
    pub contents: Vec<DeliveryContentRange>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryContentRange {
    pub asset_id: RequestId,
    pub byte_offset: ByteLength,
    pub byte_length: ByteLength,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum DeliveryInspectResponse {
    Inspected { report: Box<ReceiptInspection> },
    Error { error: crate::PptxFailure },
}
struct Range<'a, R> {
    reader: &'a R,
    start: u64,
    length: u64,
}
impl<R: ReaderAt> ReaderAt for Range<'_, R> {
    fn read_at(&self, buffer: &mut [u8], offset: u64) -> std::io::Result<usize> {
        if offset >= self.length {
            return Ok(0);
        }
        let n = (self.length - offset).min(buffer.len() as u64) as usize;
        self.reader.read_at(&mut buffer[..n], self.start + offset)
    }
}
struct Sources<'a, R>(BTreeMap<RequestId, Range<'a, R>>);
impl<R: ReaderAt> DeliverySource for Sources<'_, R> {
    fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError> {
        let data = self
            .0
            .get(id)
            .ok_or(DeliveryError::Invalid("missing delivery content range"))?;
        Ok(Content {
            reader: data,
            byte_length: data.length,
        })
    }
}
fn failure(error: DeliveryError) -> crate::PptxFailure {
    use crate::PptxFailureCode as C;
    if let DeliveryError::Pptx(e) = error {
        return crate::pptx_source::pptx_failure(e);
    }
    let code = match error {
        DeliveryError::Cancelled => C::Cancelled,
        DeliveryError::Limit(_) => C::LimitExceeded,
        DeliveryError::Io(_) => C::ReadFailed,
        _ => C::InputInvalid,
    };
    crate::PptxFailure {
        code,
        message: error.to_string(),
    }
}
fn run<R: ReaderAt>(
    input: &str,
    reader: &R,
    length: u64,
    check: &dyn Fn() -> bool,
) -> Result<ReceiptInspection, DeliveryError> {
    if check() {
        return Err(DeliveryError::Cancelled);
    }
    if input.len() > crate::MAX_REQUEST_BYTES || length > crate::MAX_INLINE_RESOURCE_BYTES as u64 {
        return Err(DeliveryError::Limit("inline delivery input"));
    }
    let request: DeliveryInspectRequest = mo_common::from_json_str(input)
        .map_err(|_| DeliveryError::Invalid("delivery request JSON"))?;
    let limits = DeliveryLimits::default();
    if request.contents.len() != request.bundle.assets.len()
        || request.contents.len() > limits.max_artifacts
    {
        return Err(DeliveryError::Invalid("delivery content coverage"));
    }
    let expected: BTreeSet<_> = request.bundle.assets.iter().map(|a| &a.id).collect();
    let mut sources = Sources(BTreeMap::new());
    let mut ranges = Vec::new();
    for binding in request.contents {
        let start = binding.byte_offset.get();
        let size = binding.byte_length.get();
        let end = start
            .checked_add(size)
            .ok_or(DeliveryError::Invalid("delivery content overflow"))?;
        if end > length
            || !expected.contains(&binding.asset_id)
            || sources
                .0
                .insert(
                    binding.asset_id,
                    Range {
                        reader,
                        start,
                        length: size,
                    },
                )
                .is_some()
        {
            return Err(DeliveryError::Invalid("delivery content binding"));
        }
        ranges.push((start, end));
    }
    ranges.sort_unstable();
    let mut end = 0;
    for (start, next) in ranges {
        if start != end {
            return Err(DeliveryError::Invalid("delivery content overlap or gap"));
        }
        end = next;
    }
    if end != length {
        return Err(DeliveryError::Invalid("unbound delivery content"));
    }
    Ok(mo_presentation_delivery::inspect(
        &request.bundle,
        &request.expected,
        &sources,
        limits,
        check,
    )?
    .report()
    .clone())
}

/// Native hosts may stream the bounded diagnostic bundle; production Embedded
/// hosts call mo_presentation_delivery::inspect with authorized asset readers.
pub fn inspect_delivery_at<R: ReaderAt>(
    input: &str,
    reader: &R,
    length: u64,
    check: &dyn Fn() -> bool,
) -> DeliveryInspectResponse {
    match run(input, reader, length, check) {
        Ok(report) => DeliveryInspectResponse::Inspected {
            report: Box::new(report),
        },
        Err(error) => DeliveryInspectResponse::Error {
            error: failure(error),
        },
    }
}
pub fn inspect_delivery_json(input: &str, bytes: &[u8]) -> String {
    serde_json::to_string(&inspect_delivery_at(
        input,
        &bytes,
        bytes.len() as u64,
        &|| false,
    ))
    .expect("typed delivery response")
}
