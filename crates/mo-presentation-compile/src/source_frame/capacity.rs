//! Measurements from the same completed frame that paints the page. These are
//! shape-local text-capacity facts, not a layout or external-application verdict.
use super::*;
use mo_presentation_source::source::SourceObjectRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PROFILE: &str = "drawingml-text-capacity-q32-v1-draft";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextCapacity {
    pub profile: String,
    /// Complete frame coverage in paint order, including empty text frames.
    pub frames: Vec<FrameCapacity>,
    /// Page-space painted text envelopes, absent from historical measurements.
    /// These are before clipping/compositing; intersections are only candidates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_ink: Option<Vec<PageTextInk>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageTextInk {
    pub object: SourceObjectRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<mo_presentation_source::source::table::SourceCellAddress>,
    /// Page-space Q32 EMU, including the recorded coordinate uncertainty.
    /// None means no nontransparent glyph outline was painted by this frame.
    pub bounds: Option<Rect>,
    /// Observations precede the native clip and later layer compositing.
    pub clipping_applied: bool,
    pub coordinate_error_bound: Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameCapacity {
    pub object: SourceObjectRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<mo_presentation_source::source::table::SourceCellAddress>,
    /// Q32 EMU before page/group/animation placement. Insets are already applied.
    pub inner: Rect,
    pub content_height: Fixed,
    /// max(content_height - inner height, 0), before vertical anchoring.
    pub vertical_excess: Fixed,
    /// Unpainted glyph ink, including overhang; not a clipping/overlap verdict.
    pub ink_bounds: Option<Rect>,
    pub line_count: u32,
    /// Counts the actual line search decisions, without rounding to integer EMU.
    pub horizontal_overflow_lines: u32,
    pub emergency_lines: u32,
    /// Largest pen excursion beyond each line's own width (indent/margins apply).
    pub maximum_left_excess: Fixed,
    pub maximum_right_excess: Fixed,
    pub first_horizontal_overflow: Option<FrameLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameLine {
    pub paragraph: u32,
    pub line: u32,
}

/// Summarize all completed lines without retaining another shaping/path graph.
/// Work is bounded by the existing frame/paragraph/glyph budgets and cancellable.
pub fn measure(
    frame: &SourceFramePlan,
    check: &dyn Fn() -> bool,
) -> Result<FrameCapacity, SourceFrameError> {
    cancel(check)?;
    let height = frame
        .region
        .inner
        .max
        .y
        .checked_sub(frame.region.inner.min.y)?;
    let mut result = FrameCapacity {
        object: frame.text.object.clone(),
        cell: frame.text.cell,
        inner: frame.region.inner,
        content_height: frame.content_height,
        vertical_excess: frame.content_height.checked_sub(height)?.max(Fixed::ZERO),
        ink_bounds: frame.bounds,
        line_count: 0,
        horizontal_overflow_lines: 0,
        emergency_lines: 0,
        maximum_left_excess: Fixed::ZERO,
        maximum_right_excess: Fixed::ZERO,
        first_horizontal_overflow: None,
    };
    for paragraph in &frame.paragraphs {
        cancel(check)?;
        let geometry = paragraph
            .computed
            .geometry
            .precise
            .as_ref()
            .ok_or(mo_text::TextError::Invalid("incomplete frame capacity"))?;
        let decisions = &paragraph.computed.geometry.paths.layout.decisions;
        if geometry.lines.len() != decisions.len()
            || geometry.lines.len() != paragraph.line_offsets.len()
        {
            return Err(mo_text::TextError::Invalid("frame capacity line coverage").into());
        }
        for (line, (geometry, decision)) in geometry.lines.iter().zip(decisions).enumerate() {
            cancel(check)?;
            result.line_count = result
                .line_count
                .checked_add(1)
                .ok_or(SourceFrameError::Limit("frame capacity line count"))?;
            let width = if line == 0 {
                paragraph.spec.widths.first
            } else {
                paragraph.spec.widths.rest
            };
            let origin = frame
                .region
                .inner
                .min
                .x
                .checked_add(paragraph.spec.left)?
                .checked_add(if line == 0 && !paragraph.spec.indent_from_right {
                    paragraph.spec.indent
                } else {
                    Fixed::ZERO
                })?;
            let alignment = paragraph.line_offsets[line].x.checked_sub(origin)?;
            result.maximum_left_excess = result
                .maximum_left_excess
                .max(Fixed::ZERO.checked_sub(geometry.pen_min.checked_add(alignment)?)?);
            result.maximum_right_excess = result.maximum_right_excess.max(
                geometry
                    .pen_max
                    .checked_add(alignment)?
                    .checked_sub(width)?,
            );
            if decision.overflows {
                result.horizontal_overflow_lines += 1;
                result.first_horizontal_overflow.get_or_insert(FrameLine {
                    paragraph: paragraph.spec.source_ordinal,
                    line: line as u32,
                });
            }
            if decision.emergency {
                result.emergency_lines += 1;
            }
        }
    }
    Ok(result)
}

/// Validate internally checkable arithmetic/coverage at transport boundaries.
/// This cannot independently prove a renderer's measurements or visual quality.
impl TextCapacity {
    pub fn validate(
        &self,
        expected_frames: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourceFrameError> {
        cancel(check)?;
        if self.profile != PROFILE || self.frames.len() != expected_frames as usize {
            return Err(mo_text::TextError::Invalid("text capacity frame coverage").into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for frame in &self.frames {
            cancel(check)?;
            let height = frame.inner.max.y.checked_sub(frame.inner.min.y)?;
            if !seen.insert((&frame.object.part, frame.object.native_id, frame.cell))
                || frame.object.part.is_empty()
                || height <= Fixed::ZERO
                || frame.inner.max.x <= frame.inner.min.x
                || frame.content_height < Fixed::ZERO
                || frame.vertical_excess
                    != frame.content_height.checked_sub(height)?.max(Fixed::ZERO)
                || frame.horizontal_overflow_lines > frame.line_count
                || frame.emergency_lines > frame.line_count
                || frame.first_horizontal_overflow.is_some()
                    != (frame.horizontal_overflow_lines != 0)
                || frame
                    .first_horizontal_overflow
                    .as_ref()
                    .is_some_and(|line| line.line >= frame.line_count)
                || frame.maximum_left_excess < Fixed::ZERO
                || frame.maximum_right_excess < Fixed::ZERO
                || frame
                    .ink_bounds
                    .is_some_and(|b| b.min.x > b.max.x || b.min.y > b.max.y)
            {
                return Err(mo_text::TextError::Invalid("text capacity measurements").into());
            }
        }
        if let Some(ink) = &self.page_ink {
            if ink.len() != self.frames.len() {
                return Err(mo_text::TextError::Invalid("page text ink coverage").into());
            }
            let mut measured = std::collections::BTreeSet::new();
            for item in ink {
                cancel(check)?;
                let key = (&item.object.part, item.object.native_id, item.cell);
                if !seen.contains(&key)
                    || !measured.insert(key)
                    || item.coordinate_error_bound < Fixed::ZERO
                    || item
                        .bounds
                        .is_some_and(|b| b.min.x > b.max.x || b.min.y > b.max.y)
                {
                    return Err(mo_text::TextError::Invalid("page text ink measurements").into());
                }
            }
        }
        Ok(())
    }
}
