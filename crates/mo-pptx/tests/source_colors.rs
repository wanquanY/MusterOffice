mod support;
use mo_common::Digest;
use mo_opc::{Package, PackageLimits};
use mo_pptx::{
    PptxError, PptxLimits, export,
    source::{color::*, theme::*, *},
};
use std::cell::Cell;

fn source() -> SourceIndex {
    let (document, defaults) = support::input();
    let bytes = export(
        &document,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    inspect_source(&package, SourceLimits::default(), &|| false).unwrap()
}
fn request(index: &SourceIndex) -> SourceColorQuery {
    SourceColorQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: "/ppt/slides/slide1.xml".into(),
        profile: ColorProfile::Ecma3762016DraftV1,
        colors: vec![SchemeColor::Accent1],
        context: ColorContext::default(),
    }
}
fn accent(index: &mut SourceIndex) -> &mut SourceColor {
    index
        .themes
        .get_mut("/ppt/theme/theme2.xml")
        .unwrap()
        .color_scheme
        .as_mut()
        .unwrap()
        .colors
        .get_mut(&ColorSlot::Accent1)
        .unwrap()
}
fn evaluate(index: &SourceIndex, req: &SourceColorQuery) -> ColorOutcome {
    query(index, req, ColorLimits::default(), &|| false)
        .unwrap()
        .colors
        .remove(0)
        .outcome
}

#[test]
fn color_budget_digest_and_cancellation_fail_atomically() {
    let index = source();
    let mut req = request(&index);
    req.colors = vec![SchemeColor::Accent1; 100];
    let ticks = Cell::new(0);
    assert!(matches!(
        query(&index, &req, ColorLimits::default(), &|| {
            ticks.set(ticks.get() + 1);
            ticks.get() > 8
        }),
        Err(PptxError::Cancelled)
    ));
    for limits in [
        ColorLimits {
            max_queries: 99,
            ..Default::default()
        },
        ColorLimits {
            max_steps: 8,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(&index, &req, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    req.expected_source_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(&index, &req, ColorLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}

#[test]
fn unresolved_context_is_not_black_and_host_system_color_has_provenance() {
    let mut index = source();
    let mut req = request(&index);
    accent(&mut index).value = SourceColorValue::System {
        color: SystemColor::WindowText,
        last_color: None,
    };
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Unresolved {
            reason: ColorUnresolved::MissingSystemColor { .. }
        }
    ));
    accent(&mut index).value = SourceColorValue::System {
        color: SystemColor::WindowText,
        last_color: Some([11, 22, 33]),
    };
    req.context
        .system_colors
        .insert(SystemColor::WindowText, [44, 55, 66]);
    let result = query(&index, &req, ColorLimits::default(), &|| false).unwrap();
    assert!(matches!(
        result.colors[0].outcome,
        ColorOutcome::Resolved {
            rgba8: [44, 55, 66, 255],
            ..
        }
    ));
    assert!(matches!(
        result.colors[0].dependencies[1],
        ColorDependency::System {
            origin: SystemColorOrigin::HostContext,
            ..
        }
    ));
    accent(&mut index).value = SourceColorValue::Scheme {
        slot: SchemeColor::Accent1,
    };
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Unresolved {
            reason: ColorUnresolved::SchemeCycle {
                slot: ColorSlot::Accent1
            }
        }
    ));
    accent(&mut index).value = SourceColorValue::Scheme {
        slot: SchemeColor::PhClr,
    };
    req.context.placeholder = Some([255, 0, 0, 128]);
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Resolved {
            rgba8: [255, 0, 0, 128],
            ..
        }
    ));
}

#[test]
fn normative_color_examples_use_distinct_channel_and_tint_spaces() {
    let mut index = source();
    let req = request(&index);
    let p = |s: &str| NativePercentage::try_from(s.to_string()).unwrap();
    let c = accent(&mut index);
    c.value = SourceColorValue::Srgb { rgb: [255, 0, 0] };
    c.transforms = vec![SourceColorTransform::RedMod(p("50000"))];
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Resolved {
            rgba8: [128, 0, 0, 255],
            ..
        }
    ));
    let c = accent(&mut index);
    c.value = SourceColorValue::Srgb { rgb: [0, 255, 0] };
    c.transforms = vec![SourceColorTransform::Tint(p("50000"))];
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Resolved {
            rgba8: [188, 255, 188, 255],
            ..
        }
    ));
    // Hue offset clamps at zero; complement rotates cyclically.
    let c = accent(&mut index);
    c.value = SourceColorValue::Srgb { rgb: [255, 0, 0] };
    c.transforms = vec![SourceColorTransform::HueOff(-5_400_000)];
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Resolved {
            rgba8: [255, 0, 0, 255],
            ..
        }
    ));
    assert_source_unchanged(&index, &req);
}
fn assert_source_unchanged(index: &SourceIndex, req: &SourceColorQuery) {
    let before = serde_json::to_vec(index).unwrap();
    query(index, req, ColorLimits::default(), &|| false).unwrap();
    assert_eq!(before, serde_json::to_vec(index).unwrap());
}

#[test]
fn numeric_range_and_resource_budget_are_distinct_from_valid_extended_gamut() {
    let mut index = source();
    let req = request(&index);
    let p = |s: &str| NativePercentage::try_from(s.to_string()).unwrap();
    accent(&mut index).value = SourceColorValue::ScRgb {
        red: p("-50%"),
        green: p("150%"),
        blue: p("0%"),
    };
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Resolved {
            rgba8: [0, 255, 0, 255],
            clipped_for_srgb: true,
            ..
        }
    ));
    assert!(matches!(
        query(
            &index,
            &req,
            ColorLimits {
                max_percentage_bytes: 3,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    accent(&mut index).value = SourceColorValue::ScRgb {
        red: p(&format!("{}%", "9".repeat(400))),
        green: p("0%"),
        blue: p("0%"),
    };
    assert!(matches!(
        evaluate(&index, &req),
        ColorOutcome::Unresolved {
            reason: ColorUnresolved::NumericRange
        }
    ));
}
