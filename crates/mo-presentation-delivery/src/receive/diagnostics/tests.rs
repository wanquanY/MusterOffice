use super::*;
use mo_geometry::{Point, Rect};
use mo_presentation_compile::source_frame::capacity::{PROFILE, TextCapacity};

pub(super) fn f(emu: i64) -> Fixed {
    Fixed::emu(Emu::new(emu))
}
pub(super) fn rect(x: i64, y: i64, width: i64, height: i64) -> Rect {
    Rect {
        min: Point { x: f(x), y: f(y) },
        max: Point {
            x: f(x + width),
            y: f(y + height),
        },
    }
}
pub(super) fn frame() -> FrameCapacity {
    FrameCapacity {
        object: SourceObjectRef {
            part: "/ppt/slides/slide1.xml".into(),
            native_id: 7,
        },
        cell: None,
        inner: rect(10, 20, 100, 80),
        content_height: f(85),
        vertical_excess: f(5),
        ink_bounds: Some(rect(12, 22, 90, 76)),
        line_count: 2,
        horizontal_overflow_lines: 0,
        emergency_lines: 0,
        maximum_left_excess: Fixed::ZERO,
        maximum_right_excess: Fixed::ZERO,
        first_horizontal_overflow: None,
    }
}
pub(super) fn page(frames: Vec<FrameCapacity>) -> PreviewMeasurements {
    PreviewMeasurements {
        page_id: SlideId::new("slide:one").unwrap(),
        evidence_asset_id: RequestId::new("evidence:one").unwrap(),
        text_capacity: Some(TextCapacity {
            profile: PROFILE.into(),
            frames,
            page_ink: None,
        }),
    }
}

#[test]
fn leading_excess_and_actual_ink_spill_remain_distinct() {
    let mut spill = frame();
    spill.object.native_id = 8;
    spill.ink_bounds = Some(rect(8, 19, 107, 87));
    let objects = BTreeMap::from([(
        (spill.object.part.clone(), 8),
        ObjectId::new("object:spill").unwrap(),
    )]);
    let report = summarize(&[page(vec![frame(), spill])], &objects, &|| false).unwrap();
    assert_eq!(report.measured_frames, 2);
    assert_eq!(report.affected_frames, 2);
    let leading = &report.findings[0];
    assert_eq!(leading.capacity_excess_emu.get(), 5);
    assert_eq!(leading.ink_excess_emu.bottom.get(), 0);
    assert!(
        leading.object_id.is_none(),
        "never guess an editable identity"
    );
    let spill = &report.findings[1];
    assert_eq!(spill.object_id.as_ref().unwrap().as_str(), "object:spill");
    assert_eq!(spill.page_number, 1);
    assert_eq!(
        [
            spill.ink_excess_emu.left.get(),
            spill.ink_excess_emu.right.get(),
            spill.ink_excess_emu.top.get(),
            spill.ink_excess_emu.bottom.get()
        ],
        [2, 5, 1, 6]
    );
    assert_eq!(spill.inner_width_emu.get(), 100);
    assert_eq!(spill.inner_height_emu.get(), 80);
}

#[test]
fn historical_unmeasured_pages_and_truncation_are_explicit() {
    let mut historical = page(vec![]);
    historical.text_capacity = None;
    let report = summarize(
        &[historical, page(vec![frame(); MAX_FINDINGS + 4])],
        &BTreeMap::new(),
        &|| false,
    )
    .unwrap();
    assert_eq!(report.unmeasured_pages, 1);
    assert_eq!(report.measured_pages, 1);
    assert_eq!(report.measured_frames, MAX_FINDINGS + 4);
    assert_eq!(report.affected_frames, MAX_FINDINGS + 4);
    assert_eq!(report.findings.len(), MAX_FINDINGS);
    assert_eq!(report.omitted_findings, 4);
    assert!(report.findings.iter().all(|v| v.page_number == 2));
    assert!(matches!(
        summarize(&[page(vec![frame()])], &BTreeMap::new(), &|| true),
        Err(DeliveryError::Cancelled)
    ));
}

#[test]
fn empty_findings_never_invent_a_quality_claim() {
    let mut fitting = frame();
    fitting.content_height = f(80);
    fitting.vertical_excess = Fixed::ZERO;
    let report = summarize(&[page(vec![fitting])], &BTreeMap::new(), &|| false).unwrap();
    assert_eq!(report.measured_frames, 1);
    assert_eq!(report.affected_frames, 0);
    let value = serde_json::to_value(report).unwrap();
    assert!(value.get("passed").is_none());
    assert!(value.get("layoutQualityProven").is_none());
}

#[test]
fn sub_emu_excess_survives_projection_and_extremes_fail_closed() {
    assert_eq!(ceil(Fixed::from_raw(1)).unwrap().get(), 1);
    assert_eq!(ceil(Fixed::from_raw(-1)).unwrap().get(), 0);
    assert_eq!(ceil(f(i64::MAX)).unwrap().get(), i64::MAX);
    assert!(ceil(Fixed::from_raw(i128::MAX)).is_err());
    assert!(difference(Fixed::from_raw(i128::MAX), Fixed::from_raw(-1)).is_err());
}

#[test]
fn long_native_addresses_cannot_expand_the_agent_summary_without_bound() {
    let mut long = frame();
    long.object.part = format!("/ppt/{}.xml", "s".repeat(MAX_FINDING_BYTES));
    let report = summarize(&[page(vec![long, frame()])], &BTreeMap::new(), &|| false).unwrap();
    assert_eq!(report.affected_frames, 2);
    assert!(report.findings.is_empty());
    assert_eq!(report.omitted_findings, 2);
    assert!(serde_json::to_vec(&report).unwrap().len() < 1024);
}
