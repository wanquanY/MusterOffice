//! Agent-facing observations, derived from the exact measured frames. This
//! performs no layout, rerendering, repair, clipping decision or claim upgrade.
use super::*;
use mo_common::{Emu, ObjectId, SlideId};
use mo_geometry::Fixed;
use mo_pptx::{PresentationPlan, source::SourceObjectRef};
use mo_presentation_compile::source_frame::capacity::FrameCapacity;
mod overlaps;
pub use overlaps::{TextInkReference, TextOverlapFinding, TextOverlapObservations};

const MAX_FINDINGS: usize = 32;
const MAX_FINDING_BYTES: usize = 24 * 1024;
type ObjectBindings = BTreeMap<(String, u32), ObjectId>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutDiagnostics {
    pub profile: String,
    pub measured_pages: usize,
    pub unmeasured_pages: usize,
    pub measured_frames: usize,
    pub affected_frames: usize,
    /// At most 32 affected frames and 24 KiB of finding JSON in page/paint order.
    /// Once either budget is exhausted the remaining findings are counted only.
    pub findings: Vec<TextLayoutFinding>,
    pub omitted_findings: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overlaps: Option<TextOverlapObservations>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextLayoutFinding {
    pub page_id: SlideId,
    /// One-based position in the delivered deck, including hidden slides.
    pub page_number: usize,
    pub evidence_asset_id: RequestId,
    /// The editable model identity, never guessed from the object's name.
    pub object_id: Option<ObjectId>,
    pub source: SourceObjectRef,
    pub cell: Option<mo_pptx::source::table::SourceCellAddress>,
    /// Shape-local dimensions after insets, rounded upward to integer EMU.
    pub inner_width_emu: Emu,
    pub inner_height_emu: Emu,
    pub content_height_emu: Emu,
    /// Includes line leading/paragraph spacing; not necessarily visible spill.
    pub capacity_excess_emu: Emu,
    /// Glyph outlines outside the inner text region. Intentional overhang and
    /// hanging punctuation can cause this; it is not a clipping/overlap verdict.
    pub ink_excess_emu: InkExcess,
    pub line_count: u32,
    pub horizontal_overflow_lines: u32,
    pub emergency_lines: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InkExcess {
    pub left: Emu,
    pub right: Emu,
    pub top: Emu,
    pub bottom: Emu,
}

pub(super) fn object_bindings(
    plan: &PresentationPlan<'_>,
    check: &dyn Fn() -> bool,
) -> Result<ObjectBindings, DeliveryError> {
    let mut result = BTreeMap::new();
    match plan {
        PresentationPlan::Author(plan) => {
            // Use the actual plan's native IDs and part declarations. Do not
            // reimplement allocation or assume that IDs are part-local/global.
            let ids: BTreeMap<_, _> = plan
                .bindings()
                .object_ids
                .iter()
                .map(|(id, native)| (*native, id))
                .collect();
            for (part, surface) in &plan.declarations().surfaces {
                cancel(check)?;
                for object in &surface.objects {
                    cancel(check)?;
                    if let Some(id) = ids.get(&object.native_id) {
                        result.insert((part.clone(), object.native_id), (*id).clone());
                    }
                }
            }
        }
        PresentationPlan::Retained { plan, .. } => {
            let bindings = plan
                .document()
                .source_bindings
                .as_ref()
                .ok_or(DeliveryError::Invalid("retained object bindings"))?;
            for (id, source) in &bindings.objects {
                cancel(check)?;
                result.insert((source.part.clone(), source.native_id), id.clone());
            }
        }
        PresentationPlan::Composed(plan) => result.extend(plan.object_bindings().clone()),
    }
    Ok(result)
}

pub(super) fn summarize(
    pages: &[PreviewMeasurements],
    objects: &ObjectBindings,
    check: &dyn Fn() -> bool,
) -> Result<LayoutDiagnostics, DeliveryError> {
    let mut report = LayoutDiagnostics {
        profile: "renderer-text-layout-observations-emu-v1-draft".into(),
        measured_pages: 0,
        unmeasured_pages: 0,
        measured_frames: 0,
        affected_frames: 0,
        findings: vec![],
        omitted_findings: 0,
        text_overlaps: Some(overlaps::summarize(pages, objects, check)?),
    };
    let mut finding_bytes = 0usize;
    let mut exhausted = false;
    for (page_number, page) in pages.iter().enumerate() {
        cancel(check)?;
        let Some(capacity) = &page.text_capacity else {
            report.unmeasured_pages += 1;
            continue;
        };
        report.measured_pages += 1;
        for frame in &capacity.frames {
            cancel(check)?;
            report.measured_frames += 1;
            let ink = ink_excess(frame)?;
            if frame.vertical_excess == Fixed::ZERO
                && frame.horizontal_overflow_lines == 0
                && frame.emergency_lines == 0
                && [ink.left, ink.right, ink.top, ink.bottom]
                    .iter()
                    .all(|v| v.get() == 0)
            {
                continue;
            }
            report.affected_frames += 1;
            if exhausted || report.findings.len() == MAX_FINDINGS {
                report.omitted_findings += 1;
                continue;
            }
            let finding = TextLayoutFinding {
                page_id: page.page_id.clone(),
                page_number: page_number + 1,
                evidence_asset_id: page.evidence_asset_id.clone(),
                object_id: objects
                    .get(&(frame.object.part.clone(), frame.object.native_id))
                    .cloned(),
                source: frame.object.clone(),
                cell: frame.cell,
                inner_width_emu: difference(frame.inner.max.x, frame.inner.min.x)?,
                inner_height_emu: difference(frame.inner.max.y, frame.inner.min.y)?,
                content_height_emu: ceil(frame.content_height)?,
                capacity_excess_emu: ceil(frame.vertical_excess)?,
                ink_excess_emu: ink,
                line_count: frame.line_count,
                horizontal_overflow_lines: frame.horizontal_overflow_lines,
                emergency_lines: frame.emergency_lines,
            };
            let bytes = serde_json::to_vec(&finding).map_err(|_| DeliveryError::Serialization)?;
            if finding_bytes + bytes.len() > MAX_FINDING_BYTES {
                exhausted = true;
                report.omitted_findings += 1;
            } else {
                finding_bytes += bytes.len();
                report.findings.push(finding);
            }
        }
    }
    Ok(report)
}

fn ink_excess(frame: &FrameCapacity) -> Result<InkExcess, DeliveryError> {
    let Some(ink) = frame.ink_bounds else {
        return Ok(InkExcess::default());
    };
    Ok(InkExcess {
        left: difference(frame.inner.min.x, ink.min.x)?,
        right: difference(ink.max.x, frame.inner.max.x)?,
        top: difference(frame.inner.min.y, ink.min.y)?,
        bottom: difference(ink.max.y, frame.inner.max.y)?,
    })
}
fn difference(a: Fixed, b: Fixed) -> Result<Emu, DeliveryError> {
    ceil(
        a.checked_sub(b)
            .map_err(|_| DeliveryError::Invalid("layout observation range"))?,
    )
}
fn ceil(value: Fixed) -> Result<Emu, DeliveryError> {
    // Never round a positive excess to zero. Division avoids overflowing the
    // Q32 numerator near i128::MAX; the public EMU domain is checked explicitly.
    let raw = value.raw().max(0);
    let unit = 1_i128 << Fixed::FRACTION_BITS;
    let whole = raw / unit + i128::from(raw % unit != 0);
    i64::try_from(whole)
        .map(Emu::new)
        .map_err(|_| DeliveryError::Invalid("layout observation EMU range"))
}

#[cfg(test)]
mod tests;
