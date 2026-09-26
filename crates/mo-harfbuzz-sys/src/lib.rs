//! Narrow native binding. Only an isolated host worker should instantiate this backend.
//! Workspace computation crates remain unsafe-free; the audited boundary is private.
#[allow(unsafe_code)]
mod ffi;
use mo_text::{
    TextError,
    backend::{self, TextBackend},
};
use std::sync::Mutex;
static INSTANCE: Mutex<()> = Mutex::new(());

#[derive(Default)]
pub struct NativeShaper {
    invalid: bool,
}
impl NativeShaper {
    pub fn is_invalid(&self) -> bool {
        self.invalid
    }
}
impl NativeShaper {
    fn execute(
        &mut self,
        font: &[u8],
        frame: &[u32],
        operation: ffi::Operation,
    ) -> Result<Vec<u32>, TextError> {
        if self.invalid || ffi::invalid() {
            return Err(TextError::Host("native instance invalidated"));
        }
        let _guard = INSTANCE
            .lock()
            .map_err(|_| TextError::Host("native instance lock poisoned"))?;
        if ffi::version() != backend::HARFBUZZ_VERSION {
            self.invalid = true;
            return Err(TextError::Host("component version mismatch"));
        }
        let shaped;
        let measured;
        let runs: Vec<_> = if !matches!(operation, ffi::Operation::Shape) {
            measured = match operation {
                ffi::Operation::Outlines => backend::decode_outline_requests(frame)?,
                _ => backend::decode_metric_requests(frame)?,
            };
            measured.iter().map(|words| (*words, None)).collect()
        } else {
            shaped = backend::decode_requests(frame)?;
            shaped
                .iter()
                .map(|run| (run.words, Some(run.language.as_str())))
                .collect()
        };
        let maximum = operation.maximum();
        let mut result = vec![0, runs.len() as u32];
        for (index, &(request, language)) in runs.iter().enumerate() {
            let (status, words) = ffi::compute(font, request, language, operation)?;
            if status != 0 {
                if status == 2 || status == 6 {
                    self.invalid = true;
                }
                return Ok(vec![status, index as u32]);
            }
            if result.len() + 1 + words.len() > maximum {
                return Ok(vec![4, index as u32]);
            }
            result
                .try_reserve(1 + words.len())
                .map_err(|_| TextError::Host("native reply allocation"))?;
            result.push(words.len() as u32);
            result.extend_from_slice(&words);
        }
        Ok(result)
    }
}
impl TextBackend for NativeShaper {
    fn shape_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.execute(font, frame, ffi::Operation::Shape)
    }
    fn measure_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.execute(font, frame, ffi::Operation::Metrics)
    }
    fn outline_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.execute(font, frame, ffi::Operation::Outlines)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
        ffi::invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creating_a_new_wrapper_cannot_clear_instance_invalidation() {
        NativeShaper::default().invalidate();
        let mut another = NativeShaper::default();
        assert!(
            another
                .shape_batch(&[], &[backend::MAGIC, 1, backend::HARFBUZZ_VERSION, 0])
                .is_err()
        );
    }
}
