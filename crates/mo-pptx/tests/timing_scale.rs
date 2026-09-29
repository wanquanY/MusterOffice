mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{source::*, timing::*, *};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn fixture() -> (String, BTreeSet<u32>, Timeline) {
    fixture_with_scale(
        ScaleValue { x: 100000, y: 0 },
        ScaleValue {
            x: 250000,
            y: 175000,
        },
        true,
    )
}
fn fixture_with_scale(
    from: ScaleValue,
    to: ScaleValue,
    with_tree: bool,
) -> (String, BTreeSet<u32>, Timeline) {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let node = TimingNode {
        restart: mo_timeline::RestartMode::Never,
        id: TimingNodeId::new("scale").unwrap(),
        start: StartCondition::Single(TimeCondition::At { offset: t(1, 4) }),
        duration: t(2, 1),
        repeat_milli: 2500.into(),
        repeat_duration: None,
        end_conditions: vec![],
        fill: FillMode::Freeze,
        time_transform: Some(TimeTransform {
            auto_reverse: true,
            ..Default::default()
        }),
        effect: Effect::Scale { target, from, to },
    };
    let timeline = Timeline {
        format: if with_tree {
            TimelineVersion::V02
        } else {
            TimelineVersion::V01
        },
        nodes: vec![node.clone()],
        tree: with_tree.then_some(TimingTree {
            roots: vec![TimingNodeId::new("sequence").unwrap()],
            containers: vec![TimingContainer {
                time_transform: None,
                presentation: None,
                navigation: None,
                restart: mo_timeline::RestartMode::Never,
                id: TimingNodeId::new("sequence").unwrap(),
                kind: ContainerKind::Sequence,
                start: StartCondition::Single(TimeCondition::At { offset: t(0, 1) }),
                end_conditions: vec![],
                duration: ContainerDuration::Automatic,
                fill: FillMode::Hold,
                children: vec![node.id],
            }],
        }),
    };
    doc.timelines.insert(slide, timeline.clone());
    let bytes = export(
        &doc,
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
    let index = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
    let part = &index.slides[0].part;
    let xml = String::from_utf8(
        package
            .read_part(&PartName::new(part).unwrap(), 1 << 20, &|| false)
            .unwrap(),
    )
    .unwrap();
    let known = index.surfaces[part]
        .objects
        .iter()
        .map(|o| o.native_id)
        .collect();
    (xml, known, timeline)
}
#[test]
fn editable_native_scale_roundtrips_in_a_time_tree_without_private_markers() {
    let (xml, known, original) = fixture();
    assert!(xml.contains("<p:animScale>"));
    assert!(xml.contains("<p:from x=\"100000\" y=\"0\"/><p:to x=\"250000\" y=\"175000\"/>"));
    let native = read_slide_timing(
        xml.as_bytes(),
        &known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(
        native.timeline.nodes[0].effect,
        Effect::Scale { .. }
    ));
    let a = TimelinePlan::compile(&original, TimelineLimits::default(), &|| false).unwrap();
    let b = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("scale-roundtrip").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        generation: PlaybackGeneration::new(1),
    };
    for at in [t(0, 1), t(1, 4), t(1, 3), t(7, 3), t(5, 1), t(20, 1)] {
        let a = a.evaluate(&binding, at, None, &|| false).unwrap();
        let b = b.evaluate(&binding, at, None, &|| false).unwrap();
        assert_eq!(
            a.state.scales.values().collect::<Vec<_>>(),
            b.state.scales.values().collect::<Vec<_>>()
        );
        assert_eq!(a.state.nodes[0].phase, b.state.nodes[0].phase);
        assert_eq!(a.state.nodes[0].progress, b.state.nodes[0].progress);
    }
}
#[test]
fn unsupported_or_invalid_native_scale_is_reported_instead_of_ignored_or_rounded() {
    let (xml, known, _) = fixture();
    for (old, new) in [
        ("<p:animScale>", "<p:animScale zoomContents=\"1\">"),
        ("<p:attrName>ScaleY</p:attrName>", ""),
        (
            "<p:attrName>ScaleX</p:attrName>",
            "<p:attrName>ppt_x</p:attrName>",
        ),
        ("<p:to x=\"250000\" y=\"175000\"/>", ""),
        ("<p:from x=\"100000\"", "<p:from x=\"-1\""),
        ("<p:from x=\"100000\"", "<p:from x=\"2147483626\""),
        ("<p:from x=\"100000\"", "<p:from x=\"100000.5\""),
        (
            "<p:to x=\"250000\" y=\"175000\"/>",
            "<p:by x=\"150000\" y=\"150000\"/>",
        ),
    ] {
        let changed = xml.replace(old, new);
        assert_ne!(changed, xml);
        assert!(
            read_slide_timing(
                changed.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false
            )
            .is_err(),
            "{new}"
        );
    }
}

#[test]
fn native_scale_input_forms_have_exact_endpoints_and_the_same_clock() {
    let (xml, known, _) = fixture();
    let points = "<p:from x=\"100000\" y=\"0\"/><p:to x=\"250000\" y=\"175000\"/>";
    let cases = [
        (
            "<p:by x=\"150%\" y=\"50%\"/>",
            [100000, 100000],
            [150000, 50000],
        ),
        ("<p:to x=\"200%\" y=\"75%\"/>", [0, 0], [200000, 75000]),
        (
            "<p:by x=\"500000\" y=\"0\"/><p:from x=\"100.0000%\" y=\"0%\"/><p:to x=\"250000\" y=\"175000\"/>",
            [100000, 0],
            [250000, 175000],
        ),
        (
            "<p:by x=\"500000\" y=\"0\"/><p:to x=\"200000\" y=\"75000\"/>",
            [0, 0],
            [200000, 75000],
        ),
        (
            "<p:from x=\"1.234%\" y=\"+100000\"/><p:to x=\"2147483.625%\" y=\"0%\"/>",
            [1234, 100000],
            [MAX_SCALE_MILLI_PERCENT, 0],
        ),
    ];
    let read = |xml: &str| {
        read_slide_timing(
            xml.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap()
        .unwrap()
    };
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("scale-input-forms").unwrap(),
        revision: Digest::from_sha256([2; 32]),
        generation: PlaybackGeneration::new(1),
    };
    for (declaration, from, to) in cases {
        let input = xml.replace(points, declaration);
        let native = read(&input);
        let expected = read(&xml.replace(
            points,
            &format!(
                "<p:from x=\"{}\" y=\"{}\"/><p:to x=\"{}\" y=\"{}\"/>",
                from[0], from[1], to[0], to[1]
            ),
        ));
        let a =
            TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
        let b = TimelinePlan::compile(&expected.timeline, TimelineLimits::default(), &|| false)
            .unwrap();
        for at in [
            t(0, 1),
            t(1, 4),
            t(1, 3),
            t(5, 4),
            t(9, 4),
            t(17, 4),
            t(20, 1),
        ] {
            assert_eq!(
                a.evaluate(&binding, at, None, &|| false).unwrap(),
                b.evaluate(&binding, at, None, &|| false).unwrap(),
                "{declaration} at {at:?}"
            );
        }
    }
}

#[test]
fn scale_points_reject_malformed_order_and_inexact_percentages() {
    let (xml, known, _) = fixture();
    let points = "<p:from x=\"100000\" y=\"0\"/><p:to x=\"250000\" y=\"175000\"/>";
    for declaration in [
        "",
        "<p:from x=\"100000\" y=\"100000\"/>",
        "<p:by x=\"150000\" y=\"150000\"/><p:by x=\"150000\" y=\"150000\"/>",
        "<p:to x=\"200000\" y=\"200000\"/><p:from x=\"100000\" y=\"100000\"/>",
        "<p:by x=\"150.0001%\" y=\"100000\"/>",
        "<p:by x=\"2147483.626%\" y=\"100000\"/>",
        "<p:by x=\"NaN\" y=\"100000\"/><p:to x=\"100000\" y=\"100000\"/>",
        "<p:by x=\"100000\"/>",
        "<p:by x=\"100000\" y=\"100000\"><p:extLst/></p:by>",
        "<p:by x=\"100000\" y=\"100000\"/>unexpected",
    ] {
        let input = xml.replace(points, declaration);
        assert!(
            read_slide_timing(
                input.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false
            )
            .is_err(),
            "{declaration}"
        );
    }
}

#[test]
fn uncalibrated_from_by_is_preserved_as_unsupported_not_guessed() {
    let (xml, known, _) = fixture();
    let input = xml.replace(
        "<p:from x=\"100000\" y=\"0\"/><p:to x=\"250000\" y=\"175000\"/>",
        "<p:by x=\"150000\" y=\"50000\"/><p:from x=\"200000\" y=\"200000\"/>",
    );
    let error = read_slide_timing(
        input.as_bytes(),
        &known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap_err();
    assert!(
        matches!(error, PptxError::Unsupported(ref message) if message.contains("from/by scale semantics require calibration"))
    );
}

#[test]
fn identity_scale_uses_editable_by_without_changing_endpoints_or_clock() {
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("native-scale-editing").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    };
    for with_tree in [false, true] {
        for to in [
            ScaleValue {
                x: 200000,
                y: 200000,
            },
            ScaleValue {
                x: 50000,
                y: 175000,
            },
            ScaleValue {
                x: 0,
                y: MAX_SCALE_MILLI_PERCENT,
            },
            ScaleValue {
                x: 100000,
                y: 100000,
            },
        ] {
            let from = ScaleValue {
                x: 100000,
                y: 100000,
            };
            let (xml, known, original) = fixture_with_scale(from, to, with_tree);
            assert!(xml.contains(&format!("<p:by x=\"{}\" y=\"{}\"/>", to.x, to.y)));
            assert!(!xml.contains("<p:from"));
            assert!(!xml.contains("<p:to "));
            let native = read_slide_timing(
                xml.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false,
            )
            .unwrap()
            .unwrap();
            match native.timeline.nodes[0].effect {
                Effect::Scale {
                    from: actual_from,
                    to: actual_to,
                    ..
                } => {
                    assert_eq!(actual_from, from);
                    assert_eq!(actual_to, to);
                }
                _ => panic!("scale effect lost"),
            }
            let a = TimelinePlan::compile(&original, TimelineLimits::default(), &|| false).unwrap();
            let b = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false)
                .unwrap();
            // Includes delay, interpolation, auto-reverse, fractional repeat,
            // final fill and backward sampling, for flat and nested owners.
            for at in [
                t(0, 1),
                t(1, 4),
                t(1, 3),
                t(5, 4),
                t(9, 4),
                t(5, 1),
                t(20, 1),
                t(1, 3),
            ] {
                let a = a.evaluate(&binding, at, None, &|| false).unwrap();
                let b = b.evaluate(&binding, at, None, &|| false).unwrap();
                assert_eq!(
                    a.state.scales.values().collect::<Vec<_>>(),
                    b.state.scales.values().collect::<Vec<_>>()
                );
                assert_eq!(a.state.nodes[0].phase, b.state.nodes[0].phase);
                assert_eq!(a.state.nodes[0].progress, b.state.nodes[0].progress);
            }
        }
    }
}

#[test]
fn nonidentity_start_is_never_rounded_or_replaced_by_implicit_identity() {
    for from in [
        ScaleValue {
            x: 100000,
            y: 99999,
        },
        ScaleValue {
            x: 100001,
            y: 100000,
        },
        ScaleValue { x: 0, y: 0 },
    ] {
        let to = ScaleValue {
            x: 200000,
            y: 50000,
        };
        let (xml, _, _) = fixture_with_scale(from, to, false);
        assert!(!xml.contains("<p:by"));
        assert!(xml.contains(&format!(
            "<p:from x=\"{}\" y=\"{}\"/><p:to x=\"200000\" y=\"50000\"/>",
            from.x, from.y
        )));
    }
}
