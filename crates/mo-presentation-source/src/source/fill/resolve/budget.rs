use super::types::*;
use crate::{
    PptxError, cancelled,
    source::{drawingml::*, fill::*},
};

pub(super) struct Budget<'a> {
    pub limits: FillResolveLimits,
    pub check: &'a dyn Fn() -> bool,
    steps: usize,
    values: usize,
    bytes: usize,
}
pub(super) trait Lexical {
    fn lexical_bytes(&self) -> usize;
}
macro_rules! scalar { ($($t:ty),+) => { $(impl Lexical for $t { fn lexical_bytes(&self) -> usize { 0 } })+ }; }
scalar!(
    bool,
    u32,
    NativeTileFlip,
    NativePathShade,
    NativePattern,
    NativeBlipCompression,
    NativeFillAlignment
);
impl Lexical for String {
    fn lexical_bytes(&self) -> usize {
        self.len()
    }
}
impl Lexical for NativePercentage {
    fn lexical_bytes(&self) -> usize {
        self.lexical().len()
    }
}
impl Lexical for NativeCoordinate {
    fn lexical_bytes(&self) -> usize {
        self.lexical().len()
    }
}
impl<'a> Budget<'a> {
    pub fn new(limits: FillResolveLimits, check: &'a dyn Fn() -> bool) -> Self {
        Self {
            limits,
            check,
            steps: 0,
            values: 0,
            bytes: 0,
        }
    }
    pub fn step(&mut self) -> Result<(), PptxError> {
        cancelled(self.check)?;
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(PptxError::Limit("fill resolution steps"))?;
        if self.steps > self.limits.max_steps {
            return Err(PptxError::Limit("fill resolution steps"));
        }
        Ok(())
    }
    pub fn values(&mut self, n: usize) -> Result<(), PptxError> {
        self.values = self
            .values
            .checked_add(n)
            .ok_or(PptxError::Limit("fill resolution values"))?;
        if self.values > self.limits.max_values {
            return Err(PptxError::Limit("fill resolution values"));
        }
        Ok(())
    }
    pub fn bytes(&mut self, n: usize) -> Result<(), PptxError> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .ok_or(PptxError::Limit("fill resolution bytes"))?;
        if self.bytes > self.limits.max_lexical_bytes {
            return Err(PptxError::Limit("fill resolution bytes"));
        }
        Ok(())
    }
    pub fn owner(&mut self, owner: &FillOwner) -> Result<FillOwner, PptxError> {
        self.step()?;
        self.values(1)?;
        self.bytes(owner.part.len())?;
        Ok(owner.clone())
    }
    pub fn origin(&mut self, origin: &FillOrigin) -> Result<FillOrigin, PptxError> {
        self.step()?;
        self.values(1)?;
        match origin {
            FillOrigin::Declaration { owner, .. } => self.bytes(owner.part.len())?,
            FillOrigin::Theme { part, via, .. } => {
                self.bytes(part.len())?;
                self.bytes(via.part.len())?;
            }
            FillOrigin::SchemaDefault { part, .. } => self.bytes(part.len())?,
            FillOrigin::ProfileDefault {} => (),
        }
        Ok(origin.clone())
    }
    pub fn at(&mut self, origin: &FillOrigin, ordinal: u32) -> Result<FillOrigin, PptxError> {
        // Charge before cloning variable source part names.
        let mut origin = self.origin(origin)?;
        match &mut origin {
            FillOrigin::Declaration { source_ordinal, .. }
            | FillOrigin::Theme { source_ordinal, .. }
            | FillOrigin::SchemaDefault { source_ordinal, .. } => *source_ordinal = ordinal,
            FillOrigin::ProfileDefault {} => (),
        }
        Ok(origin)
    }
    pub fn bind<T: Lexical + Clone>(
        &mut self,
        value: &T,
        origin: &FillOrigin,
    ) -> Result<FillValue<T>, PptxError> {
        self.values(1)?;
        self.bytes(value.lexical_bytes())?;
        let declared_by = self.origin(origin)?;
        Ok(FillValue {
            value: value.clone(),
            declared_by,
        })
    }
    pub fn color(
        &mut self,
        color: &SourceColor,
        origin: &FillOrigin,
    ) -> Result<FillColorTerm, PptxError> {
        self.step()?;
        self.values(1 + color.transforms.len())?;
        for p in color.value.percentages() {
            self.bytes(p.lexical().len())?;
        }
        for t in &color.transforms {
            self.step()?;
            if let Some(p) = t.percentage() {
                self.bytes(p.lexical().len())?;
            }
        }
        let declared_by = self.at(origin, color.source_ordinal)?;
        Ok(FillColorTerm {
            value: color.value.clone(),
            transforms: color.transforms.clone(),
            declared_by,
        })
    }
}
