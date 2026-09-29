//! Bounded broad-phase text envelope intersections, never a visibility verdict.
use super::*;
use mo_presentation_compile::source_frame::capacity::PageTextInk;

const MAX_PAIRS: usize = 1_048_576;
const MAX_OVERLAPS: usize = 16;
const MAX_BYTES: usize = 12 * 1024;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextOverlapObservations {
    pub profile: String,
    pub measured_pages: usize,
    pub unmeasured_pages: usize,
    pub checked_pairs: usize,
    pub unchecked_pairs: usize,
    /// Positive-area intersections of page-space text ink envelopes. Bounds
    /// precede clipping/compositing and may include whitespace between glyphs.
    pub intersecting_pairs: usize,
    /// At most 16 candidate pairs and 12 KiB of candidate JSON, in paint order.
    pub findings: Vec<TextOverlapFinding>,
    pub omitted_findings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextOverlapFinding {
    pub page_id: SlideId,
    pub page_number: usize,
    pub evidence_asset_id: RequestId,
    pub first: TextInkReference,
    pub second: TextInkReference,
    pub intersection_width_emu: Emu,
    pub intersection_height_emu: Emu,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextInkReference {
    pub object_id: Option<ObjectId>,
    pub source: SourceObjectRef,
    pub cell: Option<mo_pptx::source::table::SourceCellAddress>,
    pub clipping_applied: bool,
    pub coordinate_error_emu: Emu,
}

fn reference(
    ink: &PageTextInk,
    objects: &ObjectBindings,
) -> Result<TextInkReference, DeliveryError> {
    Ok(TextInkReference {
        object_id: objects
            .get(&(ink.object.part.clone(), ink.object.native_id))
            .cloned(),
        source: ink.object.clone(),
        cell: ink.cell,
        clipping_applied: ink.clipping_applied,
        coordinate_error_emu: ceil(ink.coordinate_error_bound)?,
    })
}

pub(super) fn summarize(
    pages: &[PreviewMeasurements],
    objects: &ObjectBindings,
    check: &dyn Fn() -> bool,
) -> Result<TextOverlapObservations, DeliveryError> {
    cancel(check)?;
    let mut report = TextOverlapObservations {
        profile: "page-text-ink-envelope-intersections-v1-draft".into(),
        measured_pages: 0,
        unmeasured_pages: 0,
        checked_pairs: 0,
        unchecked_pairs: 0,
        intersecting_pairs: 0,
        findings: vec![],
        omitted_findings: 0,
    };
    let mut bytes = 0;
    let mut exhausted = false;
    for (page_number, page) in pages.iter().enumerate() {
        cancel(check)?;
        let Some(ink) = page
            .text_capacity
            .as_ref()
            .and_then(|c| c.page_ink.as_ref())
        else {
            report.unmeasured_pages += 1;
            continue;
        };
        report.measured_pages += 1;
        let possible = ink
            .len()
            .checked_mul(ink.len().saturating_sub(1))
            .ok_or(DeliveryError::Invalid("text overlap pair count"))?
            / 2;
        let before = report.checked_pairs;
        'pairs: for (i, first) in ink.iter().enumerate() {
            for second in &ink[i + 1..] {
                cancel(check)?;
                if report.checked_pairs == MAX_PAIRS {
                    break 'pairs;
                }
                report.checked_pairs += 1;
                let (Some(a), Some(b)) = (first.bounds, second.bounds) else {
                    continue;
                };
                let left = a.min.x.max(b.min.x);
                let top = a.min.y.max(b.min.y);
                let right = a.max.x.min(b.max.x);
                let bottom = a.max.y.min(b.max.y);
                if right <= left || bottom <= top {
                    continue;
                }
                report.intersecting_pairs += 1;
                if exhausted || report.findings.len() == MAX_OVERLAPS {
                    report.omitted_findings += 1;
                    continue;
                }
                let finding = TextOverlapFinding {
                    page_id: page.page_id.clone(),
                    page_number: page_number + 1,
                    evidence_asset_id: page.evidence_asset_id.clone(),
                    first: reference(first, objects)?,
                    second: reference(second, objects)?,
                    intersection_width_emu: difference(right, left)?,
                    intersection_height_emu: difference(bottom, top)?,
                };
                let size = serde_json::to_vec(&finding)
                    .map_err(|_| DeliveryError::Serialization)?
                    .len();
                if size > MAX_BYTES - bytes {
                    exhausted = true;
                    report.omitted_findings += 1;
                } else {
                    bytes += size;
                    report.findings.push(finding);
                }
            }
        }
        report.unchecked_pairs = report
            .unchecked_pairs
            .checked_add(possible - (report.checked_pairs - before))
            .ok_or(DeliveryError::Invalid("text overlap pair count"))?;
    }
    Ok(report)
}
