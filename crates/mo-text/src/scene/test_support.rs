use crate::{TextError, backend::*, geometry::test_support::Backend as LayoutBackend};
#[derive(Default)]
pub(crate) struct Backend {
    pub(crate) layout: LayoutBackend,
    pub(crate) outlines: usize,
    pub(crate) missing: bool,
    pub(crate) failure: bool,
}
impl TextBackend for Backend {
    fn shape_batch(&mut self, f: &[u8], r: &[u32]) -> Result<Vec<u32>, TextError> {
        self.layout.shape_batch(f, r)
    }
    fn measure_batch(&mut self, f: &[u8], r: &[u32]) -> Result<Vec<u32>, TextError> {
        self.layout.measure_batch(f, r)
    }
    fn outline_batch(&mut self, _: &[u8], r: &[u32]) -> Result<Vec<u32>, TextError> {
        self.outlines += 1;
        if self.failure {
            return Ok(vec![2, 0]);
        }
        let requests = decode_outline_requests(r)?;
        let mut out = vec![0, requests.len() as u32];
        for r in requests {
            let ids = &r[8 + r[3] as usize * 2..];
            let mut words = vec![OUTLINES_MAGIC, 1, 1000, 64000, ids.len() as u32, 0];
            for &id in ids {
                if self.missing {
                    words.extend([id, 0, 0]);
                } else {
                    words.extend([id, 1, 4]);
                    words.extend([
                        1, 0, 0, 0, 0, 0, 0, 3, 32000, 64000, 64000, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0,
                        5, 0, 0, 0, 0, 0, 0,
                    ]);
                }
            }
            out.push(words.len() as u32);
            out.extend(words);
        }
        Ok(out)
    }
    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}
