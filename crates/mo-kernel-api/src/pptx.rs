use mo_common::{ByteLength, ResourceId, from_json_str};
use mo_opc::{ReaderAt, ResultSink, VerifiedPackage};
use mo_pptx::{ExportDefaults, PptxError, PptxLimits, ResourceData, Resources};
use mo_presentation_model::Document;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_INLINE_RESOURCE_BYTES: usize = 128 * 1024 * 1024;

/// Development bridge. Production hosts can directly supply streamed Resources.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxExportRequest {
    pub document: Document,
    pub defaults: ExportDefaults,
    pub resource_bindings: Vec<InlineResourceBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InlineResourceBinding {
    pub resource_id: ResourceId,
    pub byte_offset: ByteLength,
    pub byte_length: ByteLength,
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
struct Bundle<'a, R>(BTreeMap<ResourceId, Range<'a, R>>);
impl<R: ReaderAt> Resources for Bundle<'_, R> {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        let bytes = self
            .0
            .get(id)
            .ok_or_else(|| PptxError::ResourceRequired(id.clone()))?;
        Ok(ResourceData {
            reader: bytes,
            byte_length: bytes.length,
        })
    }
}

/// Returns bytes only after actual OPC output has been reopened. No host publication.
pub fn export_pptx_json(input: &str, resource_bytes: &[u8]) -> Result<Vec<u8>, String> {
    Ok(export_pptx_json_to(
        input,
        &resource_bytes,
        resource_bytes.len() as u64,
        Vec::new(),
        &|| false,
    )?
    .into_reader())
}

/// Development bridge using authorized range reads and private result storage.
/// Retains the existing bounded bundle contract; production hosts may supply
/// resource handles directly to mo_pptx::export_to. Does not publish a job/bundle.
pub fn export_pptx_json_to<R: ReaderAt, S: ResultSink>(
    input: &str,
    resource_reader: &R,
    resource_length: u64,
    sink: S,
    check: &dyn Fn() -> bool,
) -> Result<VerifiedPackage<S::Reader>, String> {
    if input.len() > super::MAX_REQUEST_BYTES || resource_length > MAX_INLINE_RESOURCE_BYTES as u64
    {
        return Err("LIMIT_EXCEEDED: inline PPTX export input budget".into());
    }
    let request =
        from_json_str::<PptxExportRequest>(input).map_err(|e| format!("INPUT_INVALID: {e}"))?;
    let mut bundle = Bundle(BTreeMap::new());
    for binding in request.resource_bindings {
        let start = usize::try_from(binding.byte_offset.get())
            .map_err(|_| "INPUT_INVALID: resource offset")?;
        let length = usize::try_from(binding.byte_length.get())
            .map_err(|_| "INPUT_INVALID: resource length")?;
        let end = start
            .checked_add(length)
            .ok_or("INPUT_INVALID: resource range overflow")?;
        if end as u64 > resource_length {
            return Err("INPUT_INVALID: resource range outside bundle".into());
        }
        let range = Range {
            reader: resource_reader,
            start: start as u64,
            length: length as u64,
        };
        if bundle.0.insert(binding.resource_id, range).is_some() {
            return Err("INPUT_INVALID: duplicate resource binding".into());
        }
    }
    mo_pptx::export_to(
        &request.document,
        &request.defaults,
        &bundle,
        sink,
        PptxLimits::default(),
        check,
    )
    .map_err(|e| super::pptx_source::pptx_failure(e).to_string())
}
