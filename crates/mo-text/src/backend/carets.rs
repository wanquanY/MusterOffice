use super::HARFBUZZ_VERSION;
use crate::TextError;

pub const CARETS_BATCH_MAGIC: u32 = 0x4d4f4342;
pub const CARETS_MAGIC: u32 = 0x4d4f4354;
pub const MAX_CARETS_PER_GLYPH: usize = 64;
pub const MAX_CARETS_RESULT_WORDS: usize = 2 + 64 * 7 + 4096 * (2 + MAX_CARETS_PER_GLYPH);
pub const MAX_CARETS_REQUEST_WORDS: usize = 4 + 64 * (1 + 8 + 64 * 2) + 4096;

pub fn decode_caret_requests(frame: &[u32]) -> Result<Vec<&[u32]>, TextError> {
    if frame.len() < 4
        || frame.len() > MAX_CARETS_REQUEST_WORDS
        || frame[..3] != [CARETS_BATCH_MAGIC, 1, HARFBUZZ_VERSION]
        || frame[3] > 64
    {
        return Err(TextError::BackendInvalid("caret request frame"));
    }
    let mut offset = 4;
    let mut instances = Vec::new();
    for _ in 0..frame[3] {
        let length = *frame
            .get(offset)
            .ok_or(TextError::BackendInvalid("caret instance length"))?
            as usize;
        offset += 1;
        if !(8..=392).contains(&length) {
            return Err(TextError::BackendInvalid("caret instance size"));
        }
        let words = frame
            .get(offset..offset + length)
            .ok_or(TextError::BackendInvalid("caret instance range"))?;
        instances.push(words);
        offset += length;
    }
    if offset != frame.len() {
        return Err(TextError::BackendInvalid("trailing caret request words"));
    }
    Ok(instances)
}
