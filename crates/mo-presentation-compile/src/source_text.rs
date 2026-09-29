//! Native source text to font-bound paragraphs and local glyph paths.
//! Text-frame positioning, paragraph alignment and paint belong to the page
//! compiler. This module never presents its unpainted paths as a rendered page.
mod assemble;
mod budget;
mod number;
mod script;
mod style;
mod table;
mod types;
use mo_common::Digest;
use mo_presentation_source::source::{SourceIndex, SourceObjectRef, text::cascade};
use mo_text::{backend::TextBackend, manifest::*};
pub use table::{TableTextCompiler, TableTextPreparation};
pub use types::*;

pub const PROFILE: &str = "drawingml-source-glyph-input-draft-v1";

/// Prepared plans are immutable and cannot be deserialized into executable
/// compiler state. Source text/style origins are retained separately from the
/// computation spans, which may cross otherwise identical XML run boundaries.
#[derive(Debug)]
pub struct PreparedSourceText {
    source: cascade::CascadedText,
    paragraphs: Vec<SourceParagraphPlan>,
    accounted_plan_bytes: usize,
}
impl PreparedSourceText {
    pub(crate) fn text_error(&self, paragraph: u32, error: mo_text::TextError) -> SourceTextError {
        let mo_text::TextError::FontSelection(selection) = error else {
            return error.into();
        };
        let Some(p) = self.paragraphs.get(paragraph as usize) else {
            return SourceTextError::Invalid("font diagnostic paragraph binding");
        };
        let Some(style) = p.styles.get(selection.style as usize) else {
            return SourceTextError::Invalid("font diagnostic style binding");
        };
        if style.typeface != selection.typeface || style.font_style != selection.font_style {
            return SourceTextError::Invalid("font diagnostic selection binding");
        }
        let uses: Vec<_> = p
            .fonts
            .iter()
            .filter(|f| f.style == selection.style)
            .cloned()
            .collect();
        if uses.is_empty() {
            return SourceTextError::Invalid("font diagnostic source binding");
        }
        SourceTextError::FontSelection(Box::new(SourceFontSelectionFailure {
            source_sha256: self.source.source_sha256.clone(),
            object: self.source.object.clone(),
            paragraph: self.source.paragraph_start + paragraph,
            source_ordinal: p.source_ordinal,
            selection,
            uses,
        }))
    }
    pub(crate) fn accounted_plan_bytes(&self) -> usize {
        self.accounted_plan_bytes
    }
    pub(crate) fn into_parts(self) -> (cascade::CascadedText, Vec<SourceParagraphPlan>) {
        (self.source, self.paragraphs)
    }
    pub fn source(&self) -> &cascade::CascadedText {
        &self.source
    }
    pub fn paragraphs(&self) -> &[SourceParagraphPlan] {
        &self.paragraphs
    }
    fn paragraph(&self, paragraph: u32) -> Result<&SourceParagraphPlan, SourceTextError> {
        self.paragraphs
            .get(paragraph as usize)
            .ok_or(SourceTextError::Invalid("source paragraph index"))
    }
    /// A paragraph operation, not an atomic document/page publication. The
    /// caller prepares a manifest once and keeps the backend lifecycle separate.
    pub fn shape_paragraph(
        &self,
        paragraph: u32,
        manifest: &PreparedManifest<'_, '_>,
        backend: &mut dyn TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceParagraphComputation<ManifestParagraphResult>, SourceTextError> {
        cancel(check)?;
        let result = manifest
            .shape_paragraph(self.paragraph(paragraph)?.input(), backend, check)
            .map_err(|e| self.text_error(paragraph, e))?;
        Ok(self.bind(paragraph, result))
    }
    /// Produces local, unpainted glyph paths using an explicit line-flow recipe.
    /// The future native frame compiler must derive that recipe and apply native
    /// paragraph/frame semantics, transforms and paint before page publication.
    pub fn paragraph_paths(
        &self,
        paragraph: u32,
        flow: SourceGlyphFlow,
        manifest: &PreparedManifest<'_, '_>,
        backend: &mut dyn TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceParagraphComputation<ManifestPathsResult>, SourceTextError> {
        cancel(check)?;
        let p = self.paragraph(paragraph)?;
        let result = manifest
            .paragraph_paths(
                ManifestLayoutInput {
                    paragraph: p.input(),
                    styles: &p.geometry,
                    strut_style: p.end_style,
                    spacing: flow.spacing,
                    width: flow.width,
                    overflow: flow.overflow,
                },
                flow.bounds_tolerance,
                backend,
                check,
            )
            .map_err(|e| self.text_error(paragraph, e))?;
        Ok(self.bind(paragraph, result))
    }
    fn bind<T>(&self, paragraph: u32, computation: T) -> SourceParagraphComputation<T> {
        SourceParagraphComputation {
            profile: PROFILE.into(),
            source_sha256: self.source.source_sha256.clone(),
            object: self.source.object.clone(),
            paragraph: self.source.paragraph_start + paragraph,
            source_ordinal: self.paragraphs[paragraph as usize].source_ordinal,
            computation,
        }
    }
}

/// Resolve a complete object's declarations and computation inputs before any
/// component call. No host locale, font discovery, text normalization or field
/// evaluation is implicit. Unsupported semantics remain an explicit outcome.
pub fn prepare(
    index: &SourceIndex,
    expected_source_sha256: &Digest,
    object: &SourceObjectRef,
    limits: SourceTextLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTextPreparation, SourceTextError> {
    cancel(check)?;
    let source =
        match cascade::resolve(index, expected_source_sha256, object, limits.cascade, check)? {
            cascade::TextCascadeOutcome::Cascaded { text } => *text,
            cascade::TextCascadeOutcome::Unresolved { reason } => {
                return Ok(SourceTextPreparation::Unresolved {
                    issue: SourceTextIssue::Cascade { reason },
                });
            }
        };
    let native = index.surfaces[&object.part]
        .objects
        .iter()
        .find(|o| o.native_id == object.native_id)
        .ok_or(SourceTextError::Invalid("source object binding"))?;
    prepare_bound(index, native, source, limits, check)
}
fn prepare_bound(
    index: &SourceIndex,
    native: &mo_presentation_source::source::SourceObject,
    source: cascade::CascadedText,
    limits: SourceTextLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTextPreparation, SourceTextError> {
    let body = mo_presentation_source::source::text::bind_body(
        native,
        &index.surfaces[&source.object.part].text,
        source.cell,
    )?
    .ok_or(SourceTextError::Invalid("source body binding"))?;
    if body.range.start != source.paragraph_start as usize
        || body.paragraphs.len() != source.paragraphs.len()
    {
        return Err(SourceTextError::Invalid("source paragraph binding"));
    }
    let mut paragraphs = Vec::new();
    let mut budget = budget::Budget::new(limits);
    let mut bytes = 0usize;
    for (i, runs) in body.paragraphs.iter().enumerate() {
        cancel(check)?;
        for run in runs {
            bytes = bytes
                .checked_add(run.text.len().max(1))
                .ok_or(SourceTextError::Limit("source text bytes"))?;
            if bytes > limits.max_text_bytes {
                return Err(SourceTextError::Limit("source text bytes"));
            }
        }
        match assemble::paragraph(index, &source, i as u32, runs, &mut budget, check)? {
            Ok(p) => paragraphs.push(p),
            Err(issue) => return Ok(SourceTextPreparation::Unresolved { issue }),
        }
    }
    cancel(check)?;
    Ok(SourceTextPreparation::Prepared {
        text: PreparedSourceText {
            source,
            paragraphs,
            accounted_plan_bytes: budget.accounted_bytes(),
        },
    })
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourceTextError> {
    if check() {
        Err(SourceTextError::Cancelled)
    } else {
        Ok(())
    }
}
