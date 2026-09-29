mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{source::*, timing::*, *};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;
fn p(x: &str, y: &str) -> MotionPoint {
    MotionPoint {
        x: x.to_owned().try_into().unwrap(),
        y: y.to_owned().try_into().unwrap(),
    }
}
fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn fixture(sequence: bool) -> (String, BTreeSet<u32>, Timeline) {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let effect = Effect::MotionLine {
        target: doc.slides[&slide].objects[0].clone(),
        from: p("-0.125", "0"),
        to: p("0.000000000000000001", "0.5"),
    };
    let mut timeline = if sequence {
        PresentationSequence {
            groups: vec![PresentationGroup {
                start: PresentationGroupStart::Automatic,
                batches: vec![PresentationBatch {
                    delay: t(0),
                    effects: vec![PresentationEffect {
                        id: TimingNodeId::new("line").unwrap(),
                        delay: t(250),
                        duration: t(2000),
                        repeat_milli: 1000.into(),
                        repeat_duration: None,
                        fill: FillMode::Hold,
                        time_transform: None,
                        effect,
                    }],
                }],
            }],
        }
        .compile(TimelineLimits::default(), &|| false)
        .unwrap()
    } else {
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![TimingNode {
                id: TimingNodeId::new("line").unwrap(),
                restart: RestartMode::Never,
                start: TimeCondition::At { offset: t(250) }.into(),
                duration: t(2000),
                repeat_milli: 1000.into(),
                repeat_duration: None,
                fill: FillMode::Hold,
                time_transform: None,
                end_conditions: vec![],
                effect,
            }],
        }
    };
    if let Some(tree) = &mut timeline.tree {
        for container in &mut tree.containers {
            if matches!(
                container.presentation,
                Some(PresentationRole::Effect { .. })
            ) {
                container.time_transform = Some(TimeTransform {
                    acceleration_milli_percent: 50_000,
                    deceleration_milli_percent: 50_000,
                    ..Default::default()
                });
            }
        }
    }
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
fn native_motion_is_editable_xml_and_roundtrips_exact_timing_and_coordinates() {
    for sequence in [false, true] {
        let (xml, known, timeline) = fixture(sequence);
        assert!(xml.contains("path=\"M -0.125 0 L 0.000000000000000001 0.5 E\""));
        if sequence {
            assert!(xml.contains("presetID=\"0\" presetClass=\"path\""));
        }
        let native = read_slide_timing(
            xml.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap()
        .unwrap();
        let a = TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).unwrap();
        let b =
            TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("line").unwrap(),
            revision: Digest::from_sha256([1; 32]),
            generation: PlaybackGeneration::new(1),
        };
        let history = EventHistory {
            binding: binding.clone(),
            through: t(10000),
            events: vec![],
        };
        for ms in [0, 249, 250, 751, 2250, 5000, 750, 0] {
            let a = a
                .evaluate(&binding, t(ms), Some(&history), &|| false)
                .unwrap();
            let b = b
                .evaluate(&binding, t(ms), Some(&history), &|| false)
                .unwrap();
            assert_eq!(
                a.state.motion.values().collect::<Vec<_>>(),
                b.state.motion.values().collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn unknown_path_semantics_are_rejected_before_any_visual_projection() {
    let (xml, known, _) = fixture(false);
    for (old, new) in [
        ("origin=\"layout\"", "origin=\"parent\""),
        ("origin=\"layout\"", ""),
        ("pathEditMode=\"relative\"", "pathEditMode=\"absolute\""),
        ("origin=\"layout\"", "origin=\"layout\" rAng=\"5400000\""),
        ("0.5 E", "0.5 M 1 1 L 2 2 E"),
        ("0.5 E", "0.5 Q 0 1 1 1 E"),
        ("0.5 E", "0.5e-2 E"),
        ("0.5 E", "0.0000000000000000001 E"),
        (
            "<p:attrName>ppt_y</p:attrName>",
            "<p:attrName>ppt_h</p:attrName>",
        ),
        ("</p:animMotion>", "<p:by x=\"1\" y=\"1\"/></p:animMotion>"),
    ] {
        assert!(xml.contains(old));
        assert!(
            read_slide_timing(
                xml.replace(old, new).as_bytes(),
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
fn native_container_easing_preserves_ownership_and_exact_filtered_time() {
    let (xml, known, _) = fixture(true);
    assert!(xml.contains("accel=\"50000\" decel=\"50000\""));
    let native = read_slide_timing(
        xml.as_bytes(),
        &known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap();
    assert!(
        native
            .timeline
            .nodes
            .iter()
            .all(|n| n.time_transform.is_none())
    );
    assert_eq!(
        native
            .timeline
            .tree
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .filter(|c| c.time_transform.is_some())
            .count(),
        1
    );
    let plan =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("ease").unwrap(),
        revision: Digest::from_sha256([5; 32]),
        generation: PlaybackGeneration::new(1),
    };
    let history = EventHistory {
        binding: binding.clone(),
        through: t(10000),
        events: vec![],
    };
    let f = plan
        .evaluate(&binding, t(751), Some(&history), &|| false)
        .unwrap();
    assert_eq!(
        f.state.nodes[0].progress,
        Some(ExactValue {
            numerator: "251001".into(),
            denominator: "2000000".into()
        })
    );
}
