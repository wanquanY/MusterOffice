use super::*;
use mo_text::{TextError, backend::TextBackend};
pub(crate) struct FrameBackend<'a> {
    pub inner: &'a mut dyn TextBackend,
    pub work: FrameWork,
    pub limits: SourceFrameLimits,
}
impl FrameBackend<'_> {
    fn charge(&mut self, font: &[u8], words: &[u32]) -> Result<(), TextError> {
        let limit = || TextError::Limit("native frame component work");
        let font_bytes = self
            .work
            .font_upload_bytes
            .checked_add(self.inner.font_upload_bytes(font))
            .ok_or_else(limit)?;
        let request_words = self
            .work
            .request_words
            .checked_add(words.len() as u64)
            .ok_or_else(limit)?;
        if self.work.component_calls >= self.limits.max_component_calls
            || font_bytes > self.limits.max_font_upload_bytes
            || request_words > self.limits.max_request_words
        {
            return Err(limit());
        }
        self.work.component_calls += 1;
        self.work.font_upload_bytes = font_bytes;
        self.work.request_words = request_words;
        Ok(())
    }
}
impl TextBackend for FrameBackend<'_> {
    fn shape_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.charge(font, words)?;
        self.inner.shape_batch(font, words)
    }
    fn measure_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.charge(font, words)?;
        self.inner.measure_batch(font, words)
    }
    fn outline_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.charge(font, words)?;
        self.inner.outline_batch(font, words)
    }
    fn caret_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.charge(font, words)?;
        self.inner.caret_batch(font, words)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }
}
