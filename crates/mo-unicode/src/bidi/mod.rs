//! Fixed UAX #9 / Unicode 18 paragraph resolution and explicit-line L1/L2.
//! Author text is never reordered or mirrored in storage.
mod properties;
#[cfg(test)]
mod tests;
mod types;
use crate::{TextBoundary, UnicodeError, UnicodeLimits};
use properties::DataSource18;
pub use properties::{
    BidiBracket, BidiClass, BidiProperties, bidi_class, bidi_properties, bracket,
};
pub use types::*;
use unicode_bidi::{Level, ParagraphBidiInfo};
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), BidiError> {
    if check() {
        Err(BidiError::Cancelled)
    } else {
        Ok(())
    }
}
/// Immutable paragraph resolution reused while evaluating possible line ranges.
/// Fields are private: callers cannot supply forged resolved levels.
pub struct ResolvedParagraph {
    classes: Vec<BidiClass>,
    levels: Vec<Level>,
    base: Level,
    boundaries: Vec<TextBoundary>,
}
impl ResolvedParagraph {
    pub fn prepare(
        text: &str,
        direction: ParagraphDirection,
        limits: BidiLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, BidiError> {
        resolve(
            &BidiParagraphRequest {
                text: text.into(),
                direction,
                line_ends: vec![],
            },
            limits,
            check,
        )
        .map(|v| v.0)
    }
    pub fn line(
        &self,
        start: u32,
        end: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<BidiLine, BidiError> {
        cancelled(check)?;
        if start > end
            || [start, end].iter().any(|at| {
                self.boundaries
                    .binary_search_by_key(at, |b| b.scalar_offset)
                    .is_err()
            })
        {
            return Err(BidiError::Invalid(
                "line range must follow paragraph grapheme boundaries",
            ));
        }
        line(
            start as usize,
            end as usize,
            &self.classes,
            &self.levels,
            self.base,
            &self.boundaries,
            check,
        )
    }
    pub fn paragraph_level(&self) -> u8 {
        self.base.number()
    }
    pub fn resolved_levels(&self) -> Vec<Option<u8>> {
        self.levels
            .iter()
            .zip(&self.classes)
            .map(|(level, c)| (!c.removed()).then_some(level.number()))
            .collect()
    }
}
pub fn analyze_paragraph(
    request: &BidiParagraphRequest,
    limits: BidiLimits,
    check: &dyn Fn() -> bool,
) -> Result<BidiParagraphResult, BidiError> {
    let (resolved, ends) = resolve(request, limits, check)?;
    let mut lines = Vec::with_capacity(ends.len());
    let mut start = 0;
    for end in ends {
        lines.push(resolved.line(start, end, check)?);
        start = end;
    }
    Ok(BidiParagraphResult {
        profile: "unicode18.0.0-uax9-r52-explicit-paragraph-l2-v1".into(),
        paragraph_level: resolved.paragraph_level(),
        resolved_levels: resolved.resolved_levels(),
        lines,
    })
}
fn resolve(
    request: &BidiParagraphRequest,
    limits: BidiLimits,
    check: &dyn Fn() -> bool,
) -> Result<(ResolvedParagraph, Vec<u32>), BidiError> {
    cancelled(check)?;
    if request.line_ends.len().max(1) > limits.max_lines {
        return Err(BidiError::Limit("lines"));
    }
    let segmentation = crate::segment(
        &request.text,
        UnicodeLimits {
            max_scalars: limits.max_scalars,
            max_bytes: limits.max_bytes,
        },
        check,
    )
    .map_err(|e| match e {
        UnicodeError::Cancelled => BidiError::Cancelled,
        UnicodeError::Limit(m) => BidiError::Limit(m),
    })?;
    let chars: Vec<char> = request.text.chars().collect();
    // The application supplies a paragraph, including an optional terminator.
    // Internal paragraph separators must be split by the document/plain-text layer.
    let mut classes = Vec::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        cancelled(check)?;
        let class = bidi_class(c);
        if class == BidiClass::B
            && i + 1 != chars.len()
            && !(c == '\r' && i + 2 == chars.len() && chars[i + 1] == '\n')
        {
            return Err(BidiError::Invalid(
                "multiple paragraphs in one paragraph input",
            ));
        }
        classes.push(class);
    }
    let ends = if request.line_ends.is_empty() {
        vec![chars.len() as u32]
    } else {
        request.line_ends.clone()
    };
    if ends.last().copied() != Some(chars.len() as u32)
        || (chars.is_empty() && ends != [0])
        || (!chars.is_empty() && (ends[0] == 0 || ends.windows(2).any(|w| w[0] >= w[1])))
    {
        return Err(BidiError::Invalid(
            "ordered line ends must cover the paragraph",
        ));
    }
    for end in &ends {
        cancelled(check)?;
        if segmentation
            .boundaries
            .binary_search_by_key(end, |b| b.scalar_offset)
            .is_err()
        {
            return Err(BidiError::Invalid("line end splits an extended grapheme"));
        }
    }
    let base = match request.direction {
        ParagraphDirection::AutoLeftToRight => None,
        ParagraphDirection::LeftToRight => Some(Level::ltr()),
        ParagraphDirection::RightToLeft => Some(Level::rtl()),
    };
    // The library call is synchronous and bounded by input limits. Hard external
    // cancellation still belongs to the host process/Worker, never a panic hook.
    cancelled(check)?;
    let info = ParagraphBidiInfo::new_with_data_source(&DataSource18, &request.text, base);
    cancelled(check)?;
    let scalar_levels: Vec<Level> = request
        .text
        .char_indices()
        .map(|(byte, _)| info.levels[byte])
        .collect();
    Ok((
        ResolvedParagraph {
            classes,
            levels: scalar_levels,
            base: info.paragraph_level,
            boundaries: segmentation.boundaries,
        },
        ends,
    ))
}

fn boundary(at: usize, boundaries: &[TextBoundary]) -> TextBoundary {
    boundaries[boundaries
        .binary_search_by_key(&(at as u32), |b| b.scalar_offset)
        .expect("validated line boundary")]
    .clone()
}
fn line(
    start: usize,
    end: usize,
    classes: &[BidiClass],
    all_levels: &[Level],
    base: Level,
    boundaries: &[TextBoundary],
    check: &dyn Fn() -> bool,
) -> Result<BidiLine, BidiError> {
    let mut indices = Vec::new();
    let mut levels = Vec::new();
    let mut pending = Some(0);
    // X9 entries are absent from normative L1 processing. Apply L1 only to the
    // current line, avoiding a full-paragraph clone for every line.
    for i in start..end {
        cancelled(check)?;
        let class = classes[i];
        if class.removed() {
            continue;
        }
        let at = levels.len();
        indices.push(i);
        levels.push(all_levels[i]);
        if matches!(class, BidiClass::B | BidiClass::S) {
            levels[pending.unwrap_or(at)..].fill(base);
            pending = None;
        } else if class.whitespace() {
            pending.get_or_insert(at);
        } else {
            pending = None;
        }
    }
    if let Some(at) = pending {
        levels[at..].fill(base);
    }
    let mut output_levels = vec![None; end - start];
    for (&at, level) in indices.iter().zip(&levels) {
        output_levels[at - start] = Some(level.number());
    }
    cancelled(check)?;
    let visual_order = ParagraphBidiInfo::reorder_visual(&levels)
        .into_iter()
        .map(|i| indices[i] as u32)
        .collect();
    cancelled(check)?;
    Ok(BidiLine {
        start: boundary(start, boundaries),
        end: boundary(end, boundaries),
        levels: output_levels,
        visual_order,
    })
}
