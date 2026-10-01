//! Chart label content to font-bound paragraph computation. Placement, paint,
//! legend keys and chart-frame layout are separate; local paths are not a page.
mod content;
mod prepare;
mod types;
use crate::{source_chart_labels::*, source_text::*};
use mo_common::Digest;
use mo_presentation_source::source::{SourceObjectRef, text::cascade::*};
use mo_text::{backend::TextBackend, manifest::*};
pub use prepare::prepare;
pub use types::*;
pub const PROFILE: &str = "source-chart-label-glyph-input-v1-draft";

#[derive(Debug)]
pub struct PreparedChartLabelText {
    bindings: SourceChartLabels,
    labels: Vec<ChartLabelText>,
    accounted_plan_bytes: usize,
}
impl PreparedChartLabelText {
    pub fn bindings(&self) -> &SourceChartLabels {
        &self.bindings
    }
    pub fn labels(&self) -> &[ChartLabelText] {
        &self.labels
    }
    pub fn accounted_plan_bytes(&self) -> usize {
        self.accounted_plan_bytes
    }
    fn paragraph(&self, label: u32, paragraph: u32) -> Result<&ChartTextParagraph, ChartTextError> {
        self.labels
            .get(label as usize)
            .and_then(|l| l.paragraphs.get(paragraph as usize))
            .ok_or(ChartTextError::Invalid("chart text paragraph index"))
    }
    pub fn shape_paragraph(
        &self,
        label: u32,
        paragraph: u32,
        manifest: &PreparedManifest<'_, '_>,
        backend: &mut dyn TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ChartTextComputation<ManifestParagraphResult>, ChartTextError> {
        cancel(check)?;
        let p = self.paragraph(label, paragraph)?;
        let result = manifest
            .shape_paragraph(p.computation.input(), backend, check)
            .map_err(|e| self.text_error(label, paragraph, e))?;
        Ok(self.bind(label, paragraph, result))
    }
    /// Explicit local line flow, with the same manifest, shaping, metric and
    /// outline computation as native shape/table text. No implicit system font.
    #[allow(clippy::too_many_arguments)]
    pub fn paragraph_paths(
        &self,
        label: u32,
        paragraph: u32,
        flow: SourceGlyphFlow,
        manifest: &PreparedManifest<'_, '_>,
        backend: &mut dyn TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ChartTextComputation<ManifestPathsResult>, ChartTextError> {
        cancel(check)?;
        let p = &self.paragraph(label, paragraph)?.computation;
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
            .map_err(|e| self.text_error(label, paragraph, e))?;
        Ok(self.bind(label, paragraph, result))
    }
    fn bind<T>(&self, label: u32, paragraph: u32, computation: T) -> ChartTextComputation<T> {
        ChartTextComputation {
            profile: PROFILE.into(),
            source_sha256: self.bindings.source_sha256.clone(),
            object: self.bindings.object.clone(),
            chart_part: self.bindings.chart_part.clone(),
            chart_sha256: self.bindings.chart_sha256.clone(),
            target: self.labels[label as usize].target,
            paragraph,
            computation,
        }
    }
    fn text_error(&self, label: u32, paragraph: u32, error: mo_text::TextError) -> ChartTextError {
        let mo_text::TextError::FontSelection(selection) = error else {
            return error.into();
        };
        let p = &self.labels[label as usize].paragraphs[paragraph as usize].computation;
        let Some(style) = p.styles.get(selection.style as usize) else {
            return ChartTextError::Invalid("chart diagnostic style");
        };
        if style.typeface != selection.typeface || style.font_style != selection.font_style {
            return ChartTextError::Invalid("chart diagnostic selection");
        }
        let uses: Vec<_> = p
            .fonts
            .iter()
            .filter(|f| f.style == selection.style)
            .cloned()
            .collect();
        if uses.is_empty() {
            return ChartTextError::Invalid("chart diagnostic sources");
        }
        ChartTextError::FontSelection(Box::new(self.bind(
            label,
            paragraph,
            ChartFontSelectionFailure { selection, uses },
        )))
    }
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), ChartTextError> {
    if check() {
        Err(ChartTextError::Cancelled)
    } else {
        Ok(())
    }
}
