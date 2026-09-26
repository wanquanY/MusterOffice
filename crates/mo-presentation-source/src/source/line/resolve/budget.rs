use super::{super::*, LineResolveLimits};
use crate::{PptxError, cancelled, source::drawingml::*};

pub(super) struct Budget<'a> {
    pub limits: LineResolveLimits,
    pub check: &'a dyn Fn() -> bool,
    pub steps: usize,
    pub values: usize,
    pub lexical_bytes: usize,
}
impl Budget<'_> {
    pub fn step(&mut self) -> Result<(), PptxError> {
        cancelled(self.check)?;
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(PptxError::Limit("line resolution steps"))?;
        if self.steps > self.limits.max_steps {
            return Err(PptxError::Limit("line resolution steps"));
        }
        Ok(())
    }
    fn values(&mut self, n: usize) -> Result<(), PptxError> {
        self.values = self
            .values
            .checked_add(n)
            .ok_or(PptxError::Limit("line resolution values"))?;
        if self.values > self.limits.max_values {
            return Err(PptxError::Limit("line resolution values"));
        }
        Ok(())
    }
    pub fn bytes(&mut self, n: usize) -> Result<(), PptxError> {
        self.lexical_bytes = self
            .lexical_bytes
            .checked_add(n)
            .ok_or(PptxError::Limit("line resolution bytes"))?;
        if self.lexical_bytes > self.limits.max_lexical_bytes {
            return Err(PptxError::Limit("line resolution bytes"));
        }
        Ok(())
    }
    fn percentage(&mut self, p: &NativePercentage) -> Result<(), PptxError> {
        self.bytes(p.lexical().len())
    }
    pub fn color(&mut self, color: &SourceColor) -> Result<(), PptxError> {
        self.step()?;
        self.values(1 + color.transforms.len())?;
        for p in color.value.percentages() {
            self.percentage(p)?;
        }
        for t in &color.transforms {
            self.step()?;
            if let Some(p) = t.percentage() {
                self.percentage(p)?;
            }
        }
        Ok(())
    }
    pub fn line(&mut self, line: &SourceLine) -> Result<(), PptxError> {
        self.step()?;
        self.values(16)?;
        if let Some(SourceLineFill::Solid { color: Some(c), .. }) = &line.fill {
            self.color(c)?;
        }
        if let Some(SourceLineDash::Custom { stops, .. }) = &line.dash {
            self.values(stops.len())?;
            for stop in stops {
                self.step()?;
                self.percentage(&stop.dash)?;
                self.percentage(&stop.space)?;
            }
        }
        if let Some(SourceLineJoin::Miter { limit: Some(p), .. }) = &line.join {
            self.percentage(p)?;
        }
        Ok(())
    }
}
