//! Borrowed immutable font resources, verified once and shared across operations.
//! The handle cannot be deserialized or built from unverified metadata. It grants
//! no filesystem access, embedding rights or authority to substitute resources.
use crate::{
    cascade::{CascadeFont, CascadeLimits},
    *,
};
use std::{collections::BTreeMap, sync::Arc};

struct Data<'a> {
    sources: Vec<CascadeFont>,
    fonts: Vec<VerifiedFont<'a>>,
    bindings: Vec<usize>,
    bundle_length: usize,
}
#[derive(Clone)]
pub(crate) struct FontResources<'a> {
    data: Arc<Data<'a>>,
}
impl<'a> FontResources<'a> {
    pub fn load(
        sources: &[CascadeFont],
        bundle: &'a [u8],
        limits: CascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TextError> {
        cancelled(check)?;
        if bundle.len() > limits.max_bundle_bytes || sources.len() > limits.max_font_bindings {
            return Err(TextError::Limit("font cascade resources or items"));
        }
        let mut fonts: Vec<VerifiedFont<'a>> = vec![];
        let mut bindings = vec![];
        let mut unique = BTreeMap::new();
        for source in sources {
            cancelled(check)?;
            let (offset, length, total) = (
                source.offset.get(),
                source.byte_length.get(),
                bundle.len() as u64,
            );
            if offset > total || length > total - offset {
                return Err(TextError::Invalid("font bundle range"));
            }
            let (offset, length) = (offset as usize, length as usize);
            let key = (offset, length, source.face_index);
            let index = if let Some(&index) = unique.get(&key) {
                let font: &VerifiedFont<'_> = &fonts[index];
                if font.metadata().sha256 != source.expected_sha256 {
                    return Err(FontError::ResourceConflict.into());
                }
                index
            } else {
                let font = VerifiedFont::load(
                    &source.expected_sha256,
                    source.face_index,
                    &bundle[offset..offset + length],
                    FontLimits::default(),
                    check,
                )?;
                let index = fonts.len();
                fonts.push(font);
                unique.insert(key, index);
                index
            };
            bindings.push(index);
        }
        cancelled(check)?;
        Ok(Self {
            data: Arc::new(Data {
                sources: sources.to_vec(),
                fonts,
                bindings,
                bundle_length: bundle.len(),
            }),
        })
    }
    /// Faces actually verified when this handle was loaded, not a per-operation
    /// rehash count. Cloning the handle shares those exact verified objects.
    pub fn verified_faces(&self) -> usize {
        self.data.fonts.len()
    }
    pub(crate) fn fonts(&self) -> &[VerifiedFont<'a>] {
        &self.data.fonts
    }
    pub(crate) fn bindings(&self) -> &[usize] {
        &self.data.bindings
    }
    pub(crate) fn bundle_length(&self) -> usize {
        self.data.bundle_length
    }
    fn matches(&self, sources: &[CascadeFont], check: &dyn Fn() -> bool) -> Result<(), TextError> {
        cancelled(check)?;
        if self.data.sources.len() != sources.len() {
            return Err(FontError::ResourceConflict.into());
        }
        for (a, b) in self.data.sources.iter().zip(sources) {
            cancelled(check)?;
            if a != b {
                return Err(FontError::ResourceConflict.into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ResourceInput<'r, 'data> {
    Bundle(&'data [u8]),
    Prepared(&'r FontResources<'data>),
}
impl<'data> ResourceInput<'_, 'data> {
    pub fn bundle_length(self) -> usize {
        match self {
            Self::Bundle(b) => b.len(),
            Self::Prepared(r) => r.bundle_length(),
        }
    }
    pub fn load(
        self,
        sources: &[CascadeFont],
        limits: CascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<FontResources<'data>, TextError> {
        if sources.len() > limits.max_font_bindings
            || self.bundle_length() > limits.max_bundle_bytes
        {
            return Err(TextError::Limit("font cascade resources or items"));
        }
        match self {
            Self::Bundle(b) => FontResources::load(sources, b, limits, check),
            Self::Prepared(r) => {
                r.matches(sources, check)?;
                Ok(r.clone())
            }
        }
    }
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
