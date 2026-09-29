use super::super::tests::{f, frame, page, rect};
use super::*;

fn ink(id: u32, x: i64, y: i64) -> PageTextInk {
    PageTextInk {
        object: SourceObjectRef {
            part: "/ppt/slides/slide1.xml".into(),
            native_id: id,
        },
        cell: None,
        bounds: Some(rect(x, y, 100, 20)),
        clipping_applied: false,
        coordinate_error_bound: Fixed::ZERO,
    }
}
fn measured(items: Vec<PageTextInk>) -> PreviewMeasurements {
    let mut page = page(
        items
            .iter()
            .map(|i| {
                let mut f = frame();
                f.object = i.object.clone();
                f.cell = i.cell;
                f
            })
            .collect(),
    );
    page.text_capacity.as_mut().unwrap().page_ink = Some(items);
    page
}
#[test]
fn real_page_bounds_find_text_pairs_without_counting_touching_edges() {
    let a = ink(2, 0, 0);
    let mut b = ink(3, 40, 5);
    b.clipping_applied = true;
    b.coordinate_error_bound = Fixed::from_raw(1);
    let c = ink(4, 140, 5);
    let objects = BTreeMap::from([(
        (a.object.part.clone(), 2),
        ObjectId::new("object:title").unwrap(),
    )]);
    let report = summarize(&[measured(vec![a, b, c])], &objects, &|| false).unwrap();
    assert_eq!(report.checked_pairs, 3);
    assert_eq!(report.intersecting_pairs, 1);
    let finding = &report.findings[0];
    assert_eq!(
        finding.first.object_id.as_ref().unwrap().as_str(),
        "object:title"
    );
    assert!(finding.second.object_id.is_none());
    assert!(finding.second.clipping_applied);
    assert_eq!(finding.second.coordinate_error_emu.get(), 1);
    assert_eq!(finding.intersection_width_emu.get(), 60);
    assert_eq!(finding.intersection_height_emu.get(), 15);
    assert!(
        serde_json::to_value(report)
            .unwrap()
            .get("passed")
            .is_none()
    );
}
#[test]
fn physical_cells_are_distinct_and_historical_or_empty_measurements_are_explicit() {
    let mut a = ink(2, 0, 0);
    let mut b = a.clone();
    a.cell = Some(serde_json::from_value(serde_json::json!({"row":0,"column":0})).unwrap());
    b.cell = Some(serde_json::from_value(serde_json::json!({"row":0,"column":1})).unwrap());
    let report = summarize(
        &[page(vec![]), measured(vec![]), measured(vec![a, b])],
        &BTreeMap::new(),
        &|| false,
    )
    .unwrap();
    assert_eq!(report.measured_pages, 2);
    assert_eq!(report.unmeasured_pages, 1);
    assert_eq!(report.intersecting_pairs, 1);
    assert_ne!(
        report.findings[0].first.cell,
        report.findings[0].second.cell
    );
    assert_eq!(report.findings[0].page_number, 3);
}
#[test]
fn output_is_bounded_by_count_and_bytes_but_keeps_observed_totals() {
    let items = (0..10).map(|i| ink(i, 0, 0)).collect();
    let report = summarize(&[measured(items)], &BTreeMap::new(), &|| false).unwrap();
    assert_eq!(report.intersecting_pairs, 45);
    assert_eq!(report.findings.len(), MAX_OVERLAPS);
    assert_eq!(report.omitted_findings, 45 - MAX_OVERLAPS);
    let mut a = ink(1, 0, 0);
    a.object.part = "p".repeat(MAX_BYTES);
    let report = summarize(
        &[measured(vec![a, ink(2, 0, 0), ink(3, 0, 0)])],
        &BTreeMap::new(),
        &|| false,
    )
    .unwrap();
    assert_eq!(report.intersecting_pairs, 3);
    assert_eq!(report.omitted_findings, 3);
    assert!(report.findings.is_empty());
}
#[test]
fn quadratic_candidate_work_is_bounded_and_cancellable() {
    let page = measured(
        (0..1450)
            .map(|i| {
                let mut v = ink(i, 0, 0);
                v.bounds = None;
                v
            })
            .collect(),
    );
    let report = summarize(&[page], &BTreeMap::new(), &|| false).unwrap();
    assert_eq!(report.checked_pairs, MAX_PAIRS);
    assert_eq!(report.unchecked_pairs, 1450 * 1449 / 2 - MAX_PAIRS);
    assert!(report.findings.is_empty());
    assert!(matches!(
        summarize(&[], &BTreeMap::new(), &|| true),
        Err(DeliveryError::Cancelled)
    ));
    assert!(matches!(
        summarize(
            &[measured(vec![ink(1, 0, 0), ink(2, 0, 0)])],
            &BTreeMap::new(),
            &|| true
        ),
        Err(DeliveryError::Cancelled)
    ));
}
#[test]
fn transport_rejects_duplicate_missing_or_invalid_painted_frame_measurements() {
    let page = measured(vec![ink(1, 0, 0), ink(2, 0, 0)]);
    let capacity = page.text_capacity.unwrap();
    capacity.validate(2, &|| false).unwrap();
    for mutate in 0..4 {
        let mut bad = capacity.clone();
        let ink = bad.page_ink.as_mut().unwrap();
        match mutate {
            0 => {
                ink.pop();
            }
            1 => ink[1] = ink[0].clone(),
            2 => ink[0].coordinate_error_bound = f(-1),
            _ => ink[0].bounds.as_mut().unwrap().min.x = f(101),
        }
        assert!(bad.validate(2, &|| false).is_err());
    }
}
