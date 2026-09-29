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
fn fixture() -> (String, BTreeSet<u32>) {
    fixture_with_motion(false)
}
fn point(x: &str, y: &str) -> MotionPoint {
    MotionPoint {
        x: x.to_owned().try_into().unwrap(),
        y: y.to_owned().try_into().unwrap(),
    }
}
fn fixture_with_motion(motion: bool) -> (String, BTreeSet<u32>) {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let nodes = (0..6)
        .map(|i| TimingNode {
            restart: mo_timeline::RestartMode::Never,
            id: TimingNodeId::new(format!("effect{i}")).unwrap(),
            start: match i {
                1 => StartCondition::Single(TimeCondition::After {
                    node: TimingNodeId::new("effect0").unwrap(),
                    event: NodeEvent::End,
                    delay: t(1, 4),
                }),
                2 => StartCondition::Single(TimeCondition::Click {
                    target: Some(target.clone()),
                    delay: t(1, 8),
                }),
                3 => StartCondition::Single(TimeCondition::Click {
                    target: None,
                    delay: t(0, 1),
                }),
                _ => StartCondition::Single(TimeCondition::At { offset: t(i, 4) }),
            },
            duration: t(1, 1),
            repeat_milli: if i == 4 {
                RepeatCount::Indefinite
            } else {
                2500.into()
            },
            repeat_duration: Some(RepeatDuration::Finite(t(7, 2))),
            end_conditions: vec![TimeCondition::After {
                node: TimingNodeId::new(format!("effect{i}")).unwrap(),
                event: NodeEvent::Begin,
                delay: t(13, 4),
            }],
            time_transform: Some(TimeTransform {
                speed_milli_percent: if i == 5 { -125000 } else { 100000 },
                auto_reverse: i % 2 == 0,
                acceleration_milli_percent: 25000,
                deceleration_milli_percent: 25000,
            }),
            fill: [FillMode::Freeze, FillMode::Remove, FillMode::Hold][i as usize % 3],
            effect: if motion && i >= 2 && i % 2 == 0 {
                Effect::MotionLine {
                    target: target.clone(),
                    from: point("0.1", "-0.2"),
                    to: point("0.4", "0.3"),
                }
            } else if motion && i >= 2 {
                Effect::MotionPath {
                    target: target.clone(),
                    path: MotionPath {
                        from: point("0", "0"),
                        segments: vec![
                            MotionSegment::Cubic {
                                control1: point("0.000000000000000001", "0.2"),
                                control2: point("0.3", "-0.1"),
                                to: point("0.4", "0"),
                            },
                            MotionSegment::Close,
                        ],
                    },
                }
            } else if i % 2 == 0 {
                Effect::Scale {
                    target: target.clone(),
                    from: ScaleValue {
                        x: 100000,
                        y: 50000,
                    },
                    to: ScaleValue {
                        x: 200000,
                        y: 150000,
                    },
                }
            } else {
                Effect::Rotation {
                    composition: Default::default(),
                    target: target.clone(),
                    from: -5400000,
                    to: 32400000,
                }
            },
        })
        .collect();
    doc.timelines.insert(
        slide,
        Timeline {
            format: TimelineVersion::V01,
            nodes,
            tree: None,
        },
    );
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
    (
        xml,
        index.surfaces[part]
            .objects
            .iter()
            .map(|o| o.native_id)
            .collect(),
    )
}
fn read(
    xml: &str,
    known: &BTreeSet<u32>,
    limits: TimelineLimits,
) -> Result<Option<NativeTimeline>, PptxError> {
    read_slide_timing(xml.as_bytes(), known, XmlLimits::default(), limits, &|| {
        false
    })
}

#[test]
fn presentation_envelope_keeps_the_complete_flat_graph_and_event_clock() {
    let (xml, known) = fixture();
    assert_eq!(xml.matches("nodeType=\"withEffect\"").count(), 6);
    assert_eq!(xml.matches("presetID=\"6\"").count(), 3);
    assert_eq!(xml.matches("presetID=\"8\"").count(), 3);
    assert_neutral_envelope(&xml, &known);
}

#[test]
fn motion_presets_preserve_flat_clocks_dependencies_clicks_and_exact_paths() {
    let (xml, known) = fixture_with_motion(true);
    assert_eq!(xml.matches("nodeType=\"withEffect\"").count(), 6);
    assert_eq!(xml.matches("presetID=\"6\"").count(), 1);
    assert_eq!(xml.matches("presetID=\"8\"").count(), 1);
    assert_eq!(
        xml.matches("presetID=\"0\" presetClass=\"path\"").count(),
        4
    );
    assert_neutral_envelope(&xml, &known);
    for (from, to) in [
        ("presetClass=\"path\"", "presetClass=\"emph\""),
        ("presetID=\"0\"", "presetID=\"6\""),
    ] {
        let changed = xml.replacen(from, to, 1);
        assert_ne!(xml, changed);
        assert!(read(&changed, &known, TimelineLimits::default()).is_err());
    }
}

fn assert_neutral_envelope(xml: &str, known: &BTreeSet<u32>) {
    // Independent old flat syntax: copy only behavior subtrees, not containers.
    let mut behaviors = String::new();
    let mut tail = xml;
    while let Some(start) = tail.find("<p:anim") {
        tail = &tail[start..];
        let close = if tail.starts_with("<p:animScale") {
            "</p:animScale>"
        } else if tail.starts_with("<p:animMotion") {
            "</p:animMotion>"
        } else {
            "</p:animRot>"
        };
        let end = tail.find(close).unwrap() + close.len();
        behaviors.push_str(&tail[..end]);
        tail = &tail[end..];
    }
    let flat = format!(
        "<p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst>{behaviors}</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"
    );
    let actual = read(xml, known, TimelineLimits::default())
        .unwrap()
        .unwrap();
    let expected = read(&flat, known, TimelineLimits::default())
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&actual).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    assert!(actual.timeline.tree.is_none());
    assert_eq!(
        actual.node_bindings.values().copied().collect::<Vec<_>>(),
        (2..=7).collect::<Vec<_>>()
    );
    let a = TimelinePlan::compile(&actual.timeline, TimelineLimits::default(), &|| false).unwrap();
    let b =
        TimelinePlan::compile(&expected.timeline, TimelineLimits::default(), &|| false).unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("envelope").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        generation: PlaybackGeneration::new(1),
    };
    for clicks in [
        vec![],
        vec![None],
        vec![Some(actual.timeline.nodes[0].target().clone()), None, None],
    ] {
        let history = EventHistory {
            binding: binding.clone(),
            through: t(20, 1),
            events: clicks
                .into_iter()
                .enumerate()
                .map(|(i, target)| PlaybackEvent {
                    generation: binding.generation,
                    sequence: i as u32 + 1,
                    at: t(i as i64 + 1, 2),
                    event: InputEvent::Click { target },
                })
                .collect(),
        };
        for at in [
            t(0, 1),
            t(1, 7),
            t(1, 2),
            t(5, 8),
            t(3, 2),
            t(5, 2),
            t(13, 4),
            t(7, 2),
            t(10, 1),
            t(1, 7),
        ] {
            assert_eq!(
                a.evaluate(&binding, at, Some(&history), &|| false).unwrap(),
                b.evaluate(&binding, at, Some(&history), &|| false).unwrap()
            );
        }
    }
}

#[test]
fn unmapped_editorial_content_is_not_stripped_by_tree_fallback() {
    let (xml, known) = fixture();
    for (from, to) in [
        ("nextAc=\"none\"", "nextAc=\"unknown\""),
        (
            "id=\"8\" dur=\"indefinite\"",
            "id=\"8\" repeatCount=\"2000\" dur=\"indefinite\"",
        ),
        (
            "nodeType=\"mainSeq\"",
            "nodeType=\"mainSeq\" accel=\"50000\"",
        ),
        ("presetID=\"6\"", "presetID=\"8\""),
        ("presetClass=\"emph\"", "presetClass=\"entr\""),
        ("presetSubtype=\"0\"", "presetSubtype=\"1\""),
        ("<p:seq concurrent", "<p:seq unknown=\"1\" concurrent"),
    ] {
        let changed = xml.replacen(from, to, 1);
        assert_ne!(xml, changed, "{from}");
        assert!(
            read(&changed, &known, TimelineLimits::default()).is_err(),
            "{to}"
        );
    }
}

#[test]
fn container_identities_cannot_alias_root_other_containers_or_behaviors() {
    let (xml, known) = fixture();
    for (from, to) in [
        ("<p:cTn id=\"8\"", "<p:cTn id=\"1\""),
        ("<p:cTn id=\"9\"", "<p:cTn id=\"8\""),
        ("<p:cTn id=\"2\"", "<p:cTn id=\"11\""),
    ] {
        let changed = xml.replacen(from, to, 1);
        assert_ne!(changed, xml);
        assert!(
            read(&changed, &known, TimelineLimits::default()).is_err(),
            "{to}"
        );
    }
}

#[test]
fn nonneutral_envelopes_retain_container_state_and_do_not_become_flat_timelines() {
    let (xml, known) = fixture();
    let mutations = [
        ("concurrent=\"0\"", "concurrent=\"1\""),
        ("prevAc=\"none\"", "prevAc=\"skipTimed\""),
        ("id=\"8\" dur=\"indefinite\"", "id=\"8\" dur=\"500\""),
        ("fill=\"hold\"", "fill=\"remove\""),
        ("<p:cond delay=\"0\"/>", "<p:cond delay=\"100\"/>"),
        ("nodeType=\"withEffect\"", "nodeType=\"clickEffect\""),
        (
            "</p:stCondLst>",
            "</p:stCondLst><p:endCondLst><p:cond delay=\"100\"/></p:endCondLst>",
        ),
        (
            "</p:seq>",
            "<p:nextCondLst><p:cond evt=\"onNext\" delay=\"125\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq>",
        ),
    ];
    for (case, (from, to)) in mutations.into_iter().enumerate() {
        let changed = xml.replacen(from, to, 1);
        assert_ne!(xml, changed);
        let native = read(&changed, &known, TimelineLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(native.node_bindings.len(), 15, "{to}");
        let tree = native.timeline.tree.as_ref().unwrap();
        assert_eq!(tree.containers.len(), 9);
        let main = &tree.containers[0];
        assert_eq!(native.node_bindings[&main.id], 8);
        match case {
            0 => assert!(main.navigation.as_ref().unwrap().concurrent),
            1 => assert_eq!(
                main.navigation.as_ref().unwrap().previous_action,
                PreviousAction::SkipTimed
            ),
            2 => assert_eq!(
                main.duration,
                ContainerDuration::Fixed {
                    duration: t(500, 1000)
                }
            ),
            3 => assert_eq!(main.fill, FillMode::Remove),
            4 => assert_eq!(
                main.start,
                TimeCondition::At {
                    offset: t(100, 1000)
                }
                .into()
            ),
            5 => assert_eq!(native.node_bindings[&tree.containers[3].id], 11),
            6 => assert_eq!(
                main.end_conditions,
                vec![TimeCondition::At {
                    offset: t(100, 1000)
                }]
            ),
            7 => assert_eq!(
                main.navigation.as_ref().unwrap().next_conditions,
                vec![TimeCondition::Navigation {
                    direction: NavigationDirection::Next,
                    target: None,
                    delay: t(125, 1000),
                }]
            ),
            _ => unreachable!(),
        }
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("envelope-scope").unwrap(),
            revision: Digest::from_sha256([2; 32]),
            generation: PlaybackGeneration::new(1),
        };
        let history = EventHistory {
            binding: binding.clone(),
            through: t(10, 1),
            events: vec![],
        };
        let plan =
            TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
        let frame = plan
            .evaluate(&binding, t(1, 1), Some(&history), &|| false)
            .unwrap();
        if case == 2 || case == 6 {
            let expected = ExactValue {
                numerator: "1".into(),
                denominator: if case == 2 { "2" } else { "10" }.into(),
            };
            assert_eq!(frame.state.containers[0].end, Some(expected.clone()));
            assert_eq!(frame.state.nodes[0].end, Some(expected));
        }
        if case == 4 {
            assert_eq!(
                frame.state.nodes[0].start,
                Some(ExactValue {
                    numerator: "1".into(),
                    denominator: "10".into()
                })
            );
        }
    }
}

#[test]
fn references_to_envelope_nodes_keep_their_identity_and_container_end_semantics() {
    let (xml, known) = fixture();
    for native_id in [8, 11] {
        // These containers are indefinite. Their behavior's finite end must
        // never masquerade as the referenced container's end.
        let changed = xml.replacen(
            "<p:cond evt=\"end\" delay=\"250\"><p:tn val=\"2\"/>",
            &format!("<p:cond evt=\"end\" delay=\"250\"><p:tn val=\"{native_id}\"/>"),
            1,
        );
        assert_ne!(xml, changed);
        let native = read(&changed, &known, TimelineLimits::default())
            .unwrap()
            .unwrap();
        let referenced = TimingNodeId::new(format!("tn.{native_id}")).unwrap();
        assert_eq!(native.node_bindings[&referenced], native_id);
        assert!(
            native
                .timeline
                .tree
                .as_ref()
                .unwrap()
                .containers
                .iter()
                .any(|c| c.id == referenced)
        );
        assert_eq!(
            native.timeline.nodes[1].start,
            TimeCondition::After {
                node: referenced,
                event: NodeEvent::End,
                delay: t(250, 1000)
            }
            .into()
        );
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("envelope-reference").unwrap(),
            revision: Digest::from_sha256([3; 32]),
            generation: PlaybackGeneration::new(1),
        };
        let history = EventHistory {
            binding: binding.clone(),
            through: t(20, 1),
            events: vec![],
        };
        let frame = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false)
            .unwrap()
            .evaluate(&binding, t(10, 1), Some(&history), &|| false)
            .unwrap();
        assert_eq!(frame.state.nodes[1].phase, NodePhase::Waiting);
    }
}

#[test]
fn semantic_node_budget_counts_behaviors_and_xml_capture_remains_bounded() {
    let (xml, known) = fixture();
    assert!(
        read(
            &xml,
            &known,
            TimelineLimits {
                max_nodes: 6,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(matches!(
        read(
            &xml,
            &known,
            TimelineLimits {
                max_nodes: 5,
                ..Default::default()
            }
        ),
        Err(PptxError::Limit(_))
    ));
    let oversized = xml.replacen(
        "<p:timing>",
        &format!("<p:timing>{}", "<p:extLst/>".repeat(300)),
        1,
    );
    assert!(matches!(
        read(
            &oversized,
            &known,
            TimelineLimits {
                max_nodes: 6,
                ..Default::default()
            }
        ),
        Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
    ));
}
