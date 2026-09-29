#[path = "support/rotation.rs"]
mod rotation;
use rotation::unrotated_values;
mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{source::*, timing::*, *};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;

fn time(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
fn at(n: i64) -> TimeCondition {
    TimeCondition::At { offset: time(n) }
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn fixture(with_tree: bool, alternatives: Option<usize>) -> (String, BTreeSet<u32>, Timeline) {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let mut nodes: Vec<_> = (0..3)
        .map(|i| TimingNode {
            restart: mo_timeline::RestartMode::Never,
            id: id(&format!("effect{i}")),
            start: at(0).into(),
            duration: time(2),
            end_conditions: vec![],
            repeat_milli: 1000.into(),
            repeat_duration: None,
            fill: FillMode::Hold,
            time_transform: None,
            effect: Effect::Rotation {
                composition: Default::default(),
                target: target.clone(),
                from: 0,
                to: 120,
            },
        })
        .collect();
    nodes[0].start = StartCondition::AnyOf {
        conditions: vec![
            TimeCondition::Never {},
            at(1),
            TimeCondition::Click {
                target: Some(target),
                delay: time(0),
            },
        ],
    };
    nodes[1].start = StartCondition::AnyOf {
        conditions: vec![
            at(10),
            TimeCondition::After {
                node: id("effect0"),
                event: NodeEvent::End,
                delay: time(1),
            },
        ],
    };
    nodes[1].end_conditions = vec![TimeCondition::Never {}];
    nodes[2].start = TimeCondition::Never {}.into();
    if let Some(count) = alternatives {
        nodes.truncate(1);
        nodes[0].start = StartCondition::AnyOf {
            conditions: (0..count).map(|n| at(n as i64)).collect(),
        };
    }
    let tree = with_tree.then(|| TimingTree {
        roots: vec![id("scope")],
        containers: vec![TimingContainer {
            time_transform: None,
            presentation: None,
            navigation: None,
            restart: mo_timeline::RestartMode::Never,
            id: id("scope"),
            kind: ContainerKind::Parallel,
            start: StartCondition::AnyOf {
                conditions: vec![TimeCondition::Never {}, at(2)],
            },
            duration: ContainerDuration::Fixed { duration: time(15) },
            end_conditions: vec![],
            fill: FillMode::Hold,
            children: nodes.iter().map(|n| n.id.clone()).collect(),
        }],
    });
    let timeline = Timeline {
        format: if with_tree {
            TimelineVersion::V02
        } else {
            TimelineVersion::V01
        },
        nodes,
        tree,
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
fn flat_and_tree_condition_lists_roundtrip_with_explicit_native_conditions() {
    for with_tree in [false, true] {
        let (xml, known, original) = fixture(with_tree, None);
        assert!(xml.contains("<p:stCondLst><p:cond delay=\"indefinite\"/><p:cond delay=\"1000\"/><p:cond evt=\"onClick\""));
        assert!(xml.contains("<p:endCondLst><p:cond delay=\"indefinite\"/></p:endCondLst>"));
        let native = read(&xml, &known, TimelineLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(native.timeline.nodes[0].start.conditions().len(), 3);
        assert!(matches!(
            native.timeline.nodes[2].start,
            StartCondition::Single(TimeCondition::Never {})
        ));
        if with_tree {
            assert_eq!(
                native.timeline.tree.as_ref().unwrap().containers[0]
                    .start
                    .conditions()
                    .len(),
                2
            );
        }
        let original_plan =
            TimelinePlan::compile(&original, TimelineLimits::default(), &|| false).unwrap();
        let native_plan =
            TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("native-lists").unwrap(),
            revision: Digest::from_sha256([1; 32]),
            generation: PlaybackGeneration::new(1),
        };
        for with_event in [false, true] {
            let history = |target: ObjectId| EventHistory {
                binding: binding.clone(),
                through: time(100),
                events: if with_event {
                    vec![PlaybackEvent {
                        generation: binding.generation,
                        sequence: 1,
                        at: RationalTime::new(5, 2).unwrap(),
                        event: InputEvent::Click {
                            target: Some(target),
                        },
                    }]
                } else {
                    vec![]
                },
            };
            let a = history(original.nodes[0].target().clone());
            let b = history(native.timeline.nodes[0].target().clone());
            for at in [0, 1, 2, 3, 4, 5, 10, 20, 1] {
                let a = original_plan
                    .evaluate(&binding, time(at), Some(&a), &|| false)
                    .unwrap();
                let b = native_plan
                    .evaluate(&binding, time(at), Some(&b), &|| false)
                    .unwrap();
                assert_eq!(
                    unrotated_values(&a.state, RotationBasis::Absolute),
                    unrotated_values(&b.state, RotationBasis::Layout)
                );
                assert_eq!(a.state.event_cursor, b.state.event_cursor);
                for (mut a, mut b) in a
                    .state
                    .nodes
                    .into_iter()
                    .chain(a.state.containers)
                    .zip(b.state.nodes.into_iter().chain(b.state.containers))
                {
                    a.node = id("same");
                    b.node = id("same");
                    assert_eq!(a, b);
                }
            }
        }
    }
}
#[test]
fn unknown_or_invalid_losing_alternatives_are_never_silently_discarded() {
    let (xml, known, _) = fixture(false, None);
    for replacement in [
        "<p:cond delay=\"indefinite\" evt=\"onNext\"/>",
        "<p:cond delay=\"indefinite\"><p:tn val=\"2\"/></p:cond>",
        "<p:cond delay=\"0\" evt=\"onEnd\"/>",
        "<p:cond delay=\"0\" evt=\"onBegin\"><p:tn val=\"999999\"/></p:cond>",
        "<p:cond delay=\"0\" evt=\"onMouseOver\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>",
        "<p:cond delay=\"0\" evt=\"onNext\"><p:tgtEl><p:spTgt spid=\"999999\"/></p:tgtEl></p:cond>",
        "<p:cond delay=\"0\" evt=\"onClick\"><p:tgtEl><p:spTgt spid=\"999999\"/></p:tgtEl></p:cond>",
    ] {
        let changed = xml.replacen("<p:cond delay=\"indefinite\"/>", replacement, 1);
        assert_ne!(changed, xml);
        assert!(
            read(&changed, &known, TimelineLimits::default()).is_err(),
            "{replacement}"
        );
    }
}
#[test]
fn mapped_navigation_alternatives_keep_their_direction_target_and_delay() {
    let (xml, known, _) = fixture(false, None);
    for (event, direction) in [
        ("onNext", NavigationDirection::Next),
        ("onPrev", NavigationDirection::Previous),
    ] {
        let changed = xml.replacen(
            "<p:cond delay=\"indefinite\"/>",
            &format!(
                "<p:cond delay=\"125\" evt=\"{event}\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>"
            ),
            1,
        );
        let native = read(&changed, &known, TimelineLimits::default())
            .unwrap()
            .unwrap();
        let conditions = native.timeline.nodes[0].start.conditions();
        assert_eq!(conditions.len(), 3);
        assert_eq!(
            conditions[0],
            TimeCondition::Navigation {
                direction,
                target: None,
                delay: RationalTime::new(125, 1000).unwrap()
            }
        );
        assert_eq!(
            conditions[1],
            TimeCondition::At {
                offset: RationalTime::new(1000, 1000).unwrap()
            }
        );
        assert!(matches!(
            conditions[2],
            TimeCondition::Click {
                target: Some(_),
                ..
            }
        ));
    }
}
#[test]
fn condition_budget_is_independent_of_node_count_and_checked_after_projection() {
    let (xml, known, _) = fixture(false, Some(128));
    let limits = TimelineLimits {
        max_nodes: 1,
        max_conditions: 128,
        ..Default::default()
    };
    let native = read(&xml, &known, limits).unwrap().unwrap();
    assert_eq!(native.timeline.nodes.len(), 1);
    assert_eq!(native.timeline.nodes[0].start.conditions().len(), 128);
    let oversized = xml.replacen(
        "<p:cond delay=\"0\"/>",
        &"<p:cond delay=\"0\"/>".repeat(1024),
        1,
    );
    assert!(matches!(
        read(&oversized, &known, limits),
        Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
    ));
    assert!(matches!(
        read(
            &xml,
            &known,
            TimelineLimits {
                max_conditions: 127,
                ..limits
            }
        ),
        Err(PptxError::Limit(_))
    ));
}
