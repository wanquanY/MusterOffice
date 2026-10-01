use super::*;
use mo_presentation_source::source::text::{
    cascade::{CascadedCharacterStyle, TextStyleOrigin},
    fonts::NativeTypeface,
};

pub(crate) struct Budget {
    pub limits: SourceTextLimits,
    bytes: usize,
    fonts: usize,
}
impl Budget {
    pub fn accounted_bytes(&self) -> usize {
        self.bytes
    }
    pub fn new(limits: SourceTextLimits) -> Self {
        Self {
            limits,
            bytes: 0,
            fonts: 0,
        }
    }
    pub fn charge(&mut self, amount: usize) -> Result<(), SourceTextError> {
        self.bytes = self
            .bytes
            .checked_add(amount)
            .ok_or(SourceTextError::Limit("source plan bytes"))?;
        if self.bytes > self.limits.max_plan_bytes {
            return Err(SourceTextError::Limit("source plan bytes"));
        }
        Ok(())
    }
    pub fn character(&mut self, style: &CascadedCharacterStyle) -> Result<(), SourceTextError> {
        use mo_presentation_source::source::text::NativeTextPoint;
        let a = &style.attributes;
        let mut bytes = [
            a.language.as_ref(),
            a.alternative_language.as_ref(),
            a.bookmark.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|s| s.len())
        .sum::<usize>();
        bytes += a.baseline.as_ref().map_or(0, |p| p.lexical().len());
        if let Some(NativeTextPoint::UniversalMeasure { value }) = &a.spacing {
            bytes += value.lexical().len();
        }
        for origin in style
            .origins
            .values()
            .chain(style.declarations.values().map(|d| &d.origin))
        {
            bytes += match origin {
                TextStyleOrigin::TableStyle { source, .. } => source.lexical_bytes(),
                TextStyleOrigin::Object { object, .. } => object.part.len(),
                TextStyleOrigin::Master { part, .. }
                | TextStyleOrigin::Chart { part, .. }
                | TextStyleOrigin::Presentation { part, .. }
                | TextStyleOrigin::Theme { part, .. } => part.len(),
                TextStyleOrigin::ProfileDefault {} => 0,
            };
        }
        self.charge(8192 + bytes * 4)
    }
    pub fn font(&mut self, font: &NativeTypeface) -> Result<(), SourceTextError> {
        self.fonts += 1;
        if self.fonts > self.limits.max_font_bindings {
            return Err(SourceTextError::Limit("source object font bindings"));
        }
        let path = match &font.declared_by.origin {
            TextStyleOrigin::TableStyle { source, .. } => source.lexical_bytes(),
            TextStyleOrigin::Object { object, .. } => object.part.len(),
            TextStyleOrigin::Master { part, .. }
            | TextStyleOrigin::Chart { part, .. }
            | TextStyleOrigin::Presentation { part, .. }
            | TextStyleOrigin::Theme { part, .. } => part.len(),
            TextStyleOrigin::ProfileDefault {} => 0,
        };
        let mut bytes = font.typeface.len() + path;
        for f in [font.authored_font.as_ref(), font.theme_font.as_ref()]
            .into_iter()
            .flatten()
        {
            bytes += f.typeface.len() + f.panose.as_ref().map_or(0, String::len);
        }
        bytes += font.theme.as_ref().map_or(0, |t| t.scheme.part.len());
        // Covers font bindings, cache nodes and effective-style strings, with
        // capacity headroom. Large shared declarations cannot amplify without
        // spending the object's plan budget on every retained copy.
        self.charge(2048 + bytes * 4)
    }
}
