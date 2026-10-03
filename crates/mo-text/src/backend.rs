//! Private transport between the shared Rust text engine and an isolated shaper.
//! A single call uploads one font and a batch, never one crossing per glyph.
mod carets;
mod session;
use crate::TextError;
pub use carets::*;
pub use session::FontSession;

pub const MAGIC: u32 = 0x4d4f5342;
pub const COMPONENT_MAGIC: u32 = 0x4d4f4842;
pub const HARFBUZZ_VERSION: u32 = 0x0e0500;
pub const MAX_RESULT_WORDS: usize = 2 + 256 * 9 + 262144 * 7;
pub const MAX_REQUEST_WORDS: usize = 2_200_000;

pub trait TextBackend {
    /// Actual font bytes transferred by the next batch. Legacy transports upload
    /// the whole font. A verified calculation-scoped session can reuse residency.
    fn font_upload_bytes(&self, font: &[u8]) -> u64 {
        font.len() as u64
    }
    fn supports_font_residency(&self) -> bool {
        false
    }
    fn register_font(&mut self, _font: &[u8]) -> Result<u32, TextError> {
        Err(TextError::Host("font residency unavailable"))
    }
    fn unregister_font(&mut self, _handle: u32) -> Result<(), TextError> {
        Err(TextError::Host("font residency unavailable"))
    }
    fn shape_registered(&mut self, _handle: u32, _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font residency unavailable"))
    }
    fn measure_registered(&mut self, _handle: u32, _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font residency unavailable"))
    }
    fn outline_registered(&mut self, _handle: u32, _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font residency unavailable"))
    }
    fn caret_registered(&mut self, _handle: u32, _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font caret residency unavailable"))
    }

    /// Reply: [0, count, length, component words, ...], or [status, failedRun].
    /// Any exception, trap, malformed reply or status 2/6 invalidates the instance.
    fn shape_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError>;
    fn measure_batch(&mut self, _font: &[u8], _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font measurement is unavailable"))
    }
    fn outline_batch(&mut self, _font: &[u8], _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font outlines are unavailable"))
    }
    fn caret_batch(&mut self, _font: &[u8], _frame: &[u32]) -> Result<Vec<u32>, TextError> {
        Err(TextError::Host("font carets are unavailable"))
    }
    fn invalidate(&mut self);
}

pub const METRICS_BATCH_MAGIC: u32 = 0x4d4f4d42;
pub const METRICS_MAGIC: u32 = 0x4d4f4d54;
pub const MAX_METRICS_RESULT_WORDS: usize = 2 + 256 * (1 + 6 + 28 * 3);
pub const MAX_METRICS_REQUEST_WORDS: usize = 4 + 256 * (1 + 6 + 64 * 2 + 28);
pub fn decode_metric_requests(frame: &[u32]) -> Result<Vec<&[u32]>, TextError> {
    if frame.len() < 4
        || frame.len() > MAX_METRICS_REQUEST_WORDS
        || frame[..3] != [METRICS_BATCH_MAGIC, 1, HARFBUZZ_VERSION]
        || frame[3] > 256
    {
        return Err(TextError::BackendInvalid("metric request frame"));
    }
    let mut offset = 4;
    let mut instances = Vec::new();
    for _ in 0..frame[3] {
        let length = *frame
            .get(offset)
            .ok_or(TextError::BackendInvalid("metric instance length"))?
            as usize;
        offset += 1;
        if !(6..=162).contains(&length) {
            return Err(TextError::BackendInvalid("metric instance size"));
        }
        let words = frame
            .get(offset..offset + length)
            .ok_or(TextError::BackendInvalid("metric instance range"))?;
        instances.push(words);
        offset += length;
    }
    if offset != frame.len() {
        return Err(TextError::BackendInvalid("trailing metric request words"));
    }
    Ok(instances)
}

pub const OUTLINES_BATCH_MAGIC: u32 = 0x4d4f4f42;
pub const OUTLINES_MAGIC: u32 = 0x4d4f4f54;
pub const MAX_OUTLINES_RESULT_WORDS: usize = 2 + 64 * 7 + 4096 * 3 + 262144 * 7;
pub const MAX_OUTLINES_REQUEST_WORDS: usize = 4 + 64 * (1 + 8 + 64 * 2) + 4096;
pub fn decode_outline_requests(frame: &[u32]) -> Result<Vec<&[u32]>, TextError> {
    if frame.len() < 4
        || frame.len() > MAX_OUTLINES_REQUEST_WORDS
        || frame[..3] != [OUTLINES_BATCH_MAGIC, 1, HARFBUZZ_VERSION]
        || frame[3] > 64
    {
        return Err(TextError::BackendInvalid("outline request frame"));
    }
    let mut offset = 4;
    let mut instances = Vec::new();
    for _ in 0..frame[3] {
        let length = *frame
            .get(offset)
            .ok_or(TextError::BackendInvalid("outline instance length"))?
            as usize;
        offset += 1;
        if !(8..=392).contains(&length) {
            return Err(TextError::BackendInvalid("outline instance size"));
        }
        let words = frame
            .get(offset..offset + length)
            .ok_or(TextError::BackendInvalid("outline instance range"))?;
        instances.push(words);
        offset += length;
    }
    if offset != frame.len() {
        return Err(TextError::BackendInvalid("trailing outline request words"));
    }
    Ok(instances)
}

pub struct RunFrame<'a> {
    pub language: String,
    pub words: &'a [u32],
}
/// Internal batch frame: magic, version, component version, count, then per run
/// language-byte count, request-word count, ASCII bytes as words, request words.
pub fn decode_requests(frame: &[u32]) -> Result<Vec<RunFrame<'_>>, TextError> {
    if frame.len() < 4
        || frame.len() > MAX_REQUEST_WORDS
        || frame[..3] != [MAGIC, 1, HARFBUZZ_VERSION]
        || frame[3] > 256
    {
        return Err(TextError::BackendInvalid("request frame"));
    }
    let mut offset = 4;
    let mut runs = Vec::new();
    for _ in 0..frame[3] {
        let head = frame
            .get(offset..offset + 2)
            .ok_or(TextError::BackendInvalid("request header"))?;
        let (language_len, word_len) = (head[0] as usize, head[1] as usize);
        offset += 2;
        if !(1..=255).contains(&language_len) || !(13..=MAX_REQUEST_WORDS).contains(&word_len) {
            return Err(TextError::BackendInvalid("request lengths"));
        }
        let chars = frame
            .get(offset..offset + language_len)
            .ok_or(TextError::BackendInvalid("language range"))?;
        if chars
            .iter()
            .any(|c| *c > 127 || !((*c as u8).is_ascii_alphanumeric() || *c == 45))
        {
            return Err(TextError::BackendInvalid("language bytes"));
        }
        let language = chars.iter().map(|c| *c as u8 as char).collect();
        offset += language_len;
        let words = frame
            .get(offset..offset + word_len)
            .ok_or(TextError::BackendInvalid("request words"))?;
        offset += word_len;
        runs.push(RunFrame { language, words });
    }
    if offset != frame.len() {
        return Err(TextError::BackendInvalid("trailing request words"));
    }
    Ok(runs)
}
