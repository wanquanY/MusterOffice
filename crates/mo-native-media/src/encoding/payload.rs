use crate::WorkerError;
use mo_media::{EncodeConfig, EncodeVideoFrame};
use mo_native_worker::CHUNK_BYTES;
/// Pack only active pixels; never disclose caller stride padding to the worker.
pub(super) struct Video<'a> {
    frame: EncodeVideoFrame<'a>,
    widths: [usize; 3],
    heights: [usize; 3],
    plane: usize,
    row: usize,
    column: usize,
}
impl<'a> Video<'a> {
    pub fn new(frame: EncodeVideoFrame<'a>, c: EncodeConfig) -> Result<Self, WorkerError> {
        let widths = [c.width as usize, c.width as usize / 2, c.width as usize / 2];
        let heights = [
            c.height as usize,
            c.height as usize / 2,
            c.height as usize / 2,
        ];
        for p in 0..3 {
            let required = heights[p]
                .checked_sub(1)
                .and_then(|n| n.checked_mul(frame.strides[p]))
                .and_then(|n| n.checked_add(widths[p]));
            if frame.strides[p] < widths[p] || required.is_none_or(|n| n > frame.planes[p].len()) {
                return Err(WorkerError::Invalid("encoder plane stride or extent"));
            }
        }
        Ok(Self {
            frame,
            widths,
            heights,
            plane: 0,
            row: 0,
            column: 0,
        })
    }
}
impl Iterator for Video<'_> {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.plane == 3 {
            return None;
        }
        let mut bytes = Vec::with_capacity(CHUNK_BYTES);
        while self.plane < 3 && bytes.len() < CHUNK_BYTES {
            let p = self.plane;
            let n = (self.widths[p] - self.column).min(CHUNK_BYTES - bytes.len());
            let offset = self.row * self.frame.strides[p] + self.column;
            bytes.extend_from_slice(&self.frame.planes[p][offset..offset + n]);
            self.column += n;
            if self.column == self.widths[p] {
                self.column = 0;
                self.row += 1;
            }
            if self.row == self.heights[p] {
                self.row = 0;
                self.plane += 1;
            }
        }
        Some(bytes)
    }
}
