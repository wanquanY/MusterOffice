//! Shared validated resources and accounting for atomic and mixed-font shaping.
use super::*;
use crate::resources::{FontResources, ResourceInput};
pub(crate) struct Context<'a> {
    text: &'a str,
    resources: FontResources<'a>,
    pub scalars: Vec<char>,
    pub boundaries: BTreeSet<u32>,
    pub limits: CascadeLimits,
    pub shaping_runs: u32,
    pub component_calls: u32,
    pub context_scalars: u32,
    pub probed_glyphs: u32,
    byte_offsets: Vec<usize>,
    scope: std::ops::Range<usize>,
}
impl<'a> Context<'a> {
    pub fn fonts(&self) -> &[VerifiedFont<'a>] {
        self.resources.fonts()
    }
    pub fn bindings(&self) -> &[usize] {
        self.resources.bindings()
    }
    pub fn prepare(
        request: &'a CascadeRequest,
        bundle: &'a [u8],
        limits: CascadeLimits,
        check: &dyn Fn() -> bool,
        validate_styles: impl FnOnce(&[VerifiedFont<'_>], &[usize]) -> Result<(), TextError>,
    ) -> Result<Self, TextError> {
        Self::prepare_using(
            request,
            ResourceInput::Bundle(bundle),
            limits,
            check,
            validate_styles,
        )
    }
    pub fn prepare_using(
        request: &'a CascadeRequest,
        input: ResourceInput<'_, 'a>,
        limits: CascadeLimits,
        check: &dyn Fn() -> bool,
        validate_styles: impl FnOnce(&[VerifiedFont<'_>], &[usize]) -> Result<(), TextError>,
    ) -> Result<Self, TextError> {
        cancelled(check)?;
        if input.bundle_length() > limits.max_bundle_bytes
            || request.fonts.len() > limits.max_font_bindings
            || request.items.len() > limits.max_items
        {
            return Err(TextError::Limit("font cascade resources or items"));
        }
        let segmentation = mo_unicode::segment(&request.text, UnicodeLimits::default(), check)
            .map_err(unicode_error)?;
        let scalars: Vec<char> = request.text.chars().collect();
        let boundaries: BTreeSet<u32> = segmentation
            .boundaries
            .iter()
            .map(|v| v.scalar_offset)
            .collect();
        let mut previous_end = 0;
        for item in &request.items {
            cancelled(check)?;
            if item.start >= item.end
                || item.start < previous_end
                || !boundaries.contains(&item.start)
                || !boundaries.contains(&item.end)
                || item.candidates.is_empty()
            {
                return Err(TextError::Invalid(
                    "ordered nonempty grapheme-aligned items and candidates",
                ));
            }
            if item.candidates.len() > limits.max_candidates_per_item {
                return Err(TextError::Limit("font candidates per item"));
            }
            if item
                .candidates
                .iter()
                .any(|c| c.font as usize >= request.fonts.len())
            {
                return Err(TextError::Invalid("font candidate binding"));
            }
            previous_end = item.end;
        }
        let resources = input.load(&request.fonts, limits, check)?;
        let (fonts, bindings) = (resources.fonts(), resources.bindings());
        validate_styles(fonts, bindings)?;
        // Validate even later/unused candidates before a component is invoked. A bad
        // axis or run cannot be hidden by an earlier font happening to work today.
        for item in &request.items {
            for candidate in &item.candidates {
                cancelled(check)?;
                let font = &fonts[bindings[candidate.font as usize]];
                prepare::validate_run(
                    &item.run(candidate),
                    font.metadata(),
                    scalars.len(),
                    TextLimits::default(),
                )?;
            }
        }
        let scope = 0..scalars.len();
        let byte_offsets = request
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(request.text.len()))
            .collect();
        Ok(Self {
            text: &request.text,
            resources,
            scalars,
            boundaries,
            limits,
            shaping_runs: 0,
            component_calls: 0,
            context_scalars: 0,
            probed_glyphs: 0,
            byte_offsets,
            scope,
        })
    }
    /// Caller validates every scope before any component call. Source ranges
    /// remain absolute; only the isolated shaping context is clipped to a line.
    pub fn set_scope(&mut self, scope: std::ops::Range<u32>) {
        self.scope = scope.start as usize..scope.end as usize;
    }
    pub fn shape(
        &mut self,
        candidate: &FontCandidate,
        mut runs: Vec<ShapeRun>,
        backend: &mut dyn TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ShapedText, TextError> {
        cancelled(check)?;
        let count = runs.len();
        let context = self
            .scope
            .len()
            .checked_mul(count)
            .ok_or(TextError::Limit("font cascade shaping context"))?;
        if u64::from(self.shaping_runs) + count as u64
            > self.limits.max_attempts.min(u32::MAX as usize) as u64
            || u64::from(self.context_scalars) + context as u64
                > self.limits.max_context_scalars.min(u32::MAX as usize) as u64
        {
            return Err(TextError::Limit("font cascade attempts or shaping context"));
        }
        let font = &self.fonts()[self.bindings()[candidate.font as usize]];
        let start = self.scope.start as u32;
        let end = self.scope.end as u32;
        for run in &mut runs {
            if run.start < start || run.end > end {
                return Err(TextError::Invalid("shaping run outside context scope"));
            }
            if start != 0 || end as usize != self.scalars.len() {
                run.start -= start;
                run.end -= start;
                run.features = run
                    .features
                    .iter()
                    .filter_map(|f| {
                        let a = f.start.max(start);
                        let b = f.end.unwrap_or(self.scalars.len() as u32).min(end);
                        (a < b).then(|| ShapeFeature {
                            tag: f.tag.clone(),
                            value: f.value,
                            start: a - start,
                            end: Some(b - start),
                        })
                    })
                    .collect();
            }
        }
        let request = ShapeRequest {
            text: self.text[self.byte_offsets[self.scope.start]..self.byte_offsets[self.scope.end]]
                .into(),
            expected_sha256: font.metadata().sha256.clone(),
            face_index: font.metadata().face_index,
            runs,
        };
        let mut shaped = shape_verified(&request, font, backend, TextLimits::default(), check)?;
        if start != 0 {
            for run in &mut shaped.runs {
                run.start += start;
                run.end += start;
                for cluster in &mut run.missing_glyph_clusters {
                    *cluster += start;
                }
                for glyph in &mut run.glyphs {
                    glyph.cluster += start;
                }
            }
        }
        self.shaping_runs += count as u32;
        self.component_calls += 1;
        self.context_scalars += context as u32;
        let glyphs: usize = shaped.runs.iter().map(|r| r.glyphs.len()).sum();
        if u64::from(self.probed_glyphs) + glyphs as u64
            > self.limits.max_probed_glyphs.min(u32::MAX as usize) as u64
        {
            return Err(TextError::Limit("font cascade probed glyphs"));
        }
        self.probed_glyphs += glyphs as u32;
        Ok(shaped)
    }
    pub fn variations(
        &self,
        candidate: &FontCandidate,
        item: &CascadeItem,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<VariationIssue>, TextError> {
        variation_issues(
            &self.fonts()[self.bindings()[candidate.font as usize]],
            &self.scalars,
            item,
            check,
        )
    }
}
