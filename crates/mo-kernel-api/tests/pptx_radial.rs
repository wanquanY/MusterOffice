#[allow(dead_code)]
#[path = "../../../tools/test-support/radial_layout.rs"]
mod support;
use mo_kernel_api::*;
use support::*;
fn request(source: &[u8], targets: Vec<SourceFillTarget>) -> PptxRadialLayoutRequest {
    let index = read(source);
    PptxRadialLayoutRequest {
        profile: RadialLayoutProfile::DrawingmlCircleDraftV1,
        fills: SourceFillQuery {
            expected_source_sha256: index.source_sha256,
            surface: SLIDE.into(),
            targets,
            profile: mo_pptx::source::fill::resolve::FillProfile::Drawingml2024DraftV1,
        },
        options: RadialLayoutOptions {
            coordinate_tolerance: mo_geometry::Fixed::from_raw(1 << 24),
        },
    }
}
use mo_pptx::source::fill::resolve::FillTarget as SourceFillTarget;
#[test]
fn radial_public_api_diagnostics_keep_cancel_limits_and_conflicts() {
    let c = radial_cases().remove(0);
    let q = request(&c.source, c.targets);
    let run = |request: &PptxRadialLayoutRequest, limits, cancel: &dyn Fn() -> bool| {
        layout_pptx_radial(
            request,
            c.source.as_slice(),
            c.source.len() as u64,
            SourceLimits::default(),
            limits,
            cancel,
        )
    };
    for (response, expected) in [
        (
            run(&q, RadialLayoutLimits::default(), &|| true),
            PptxFailureCode::Cancelled,
        ),
        (
            run(
                &q,
                RadialLayoutLimits {
                    max_bounds_steps: 0,
                    ..Default::default()
                },
                &|| false,
            ),
            PptxFailureCode::LimitExceeded,
        ),
    ] {
        let PptxRadialLayoutResponse::Error { error } = response else {
            panic!("expected failure")
        };
        assert_eq!(
            serde_json::to_value(error.code).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
    let other = read(&image_fixture(""));
    let mut bad = q.clone();
    bad.fills.expected_source_sha256 = other.source_sha256;
    let PptxRadialLayoutResponse::Error { error } =
        run(&bad, RadialLayoutLimits::default(), &|| false)
    else {
        panic!()
    };
    assert!(matches!(error.code, PptxFailureCode::SourceConflict));
}
#[test]
fn radial_public_json_is_atomic_and_rejects_ambiguous_requests() {
    let good = circle("l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"");
    let bad = circle("l=\"80%\" r=\"80%\"");
    let source = image_fixture(
        &(receiver(42, [0, 0, 600000, 800000], "", &good)
            + &receiver(43, [600000, 0, 600000, 800000], "", &bad)),
    );
    let q = request(
        &source,
        vec![
            SourceFillTarget::Object { native_id: 42 },
            SourceFillTarget::Object { native_id: 43 },
        ],
    );
    let json = serde_json::to_string(&q).unwrap();
    let result: serde_json::Value =
        serde_json::from_str(&layout_pptx_radial_json(&json, &source)).unwrap();
    assert_eq!(result["status"], "error");
    assert!(result.get("plans").is_none());
    assert!(result["error"]["message"].as_str().unwrap().contains("43"));
    let mut q = serde_json::to_value(q).unwrap();
    q["fills"]["targets"].as_array_mut().unwrap().pop();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&layout_pptx_radial_json(
            &q.to_string(),
            &source
        ))
        .unwrap()["status"],
        "evaluated"
    );
    for value in [
        "{\"profile\":0,\"profile\":0}".to_string(),
        q.to_string()
            .replace("drawingml-circle-path-bounds-q96-v1-draft", "unknown"),
    ] {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&layout_pptx_radial_json(&value, &source))
                .unwrap()["status"],
            "error"
        );
    }
}
