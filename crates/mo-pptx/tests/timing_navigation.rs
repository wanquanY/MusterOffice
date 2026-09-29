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

fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn time(n: i64) -> RationalTime {
    RationalTime::new(n, 1000).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("native-navigation").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn input(events: &[(i64, NavigationDirection)]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: time(10000),
        events: events
            .iter()
            .enumerate()
            .map(|(i, (at, direction))| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: time(*at),
                event: InputEvent::Navigation {
                    direction: *direction,
                    target: None,
                },
            })
            .collect(),
    }
}
fn read(xml: &[u8], known: &BTreeSet<u32>) -> Result<Option<NativeTimeline>, PptxError> {
    read_slide_timing(
        xml,
        known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
}
fn fixture(concurrent: bool) -> (String, BTreeSet<u32>, Timeline) {
    fixture_with_action(concurrent, NextAction::None)
}
fn fixture_with_action(
    concurrent: bool,
    next_action: NextAction,
) -> (String, BTreeSet<u32>, Timeline) {
    let (mut document, defaults) = support::input();
    let slide = document.slide_order[0].clone();
    let target = document.slides[&slide].objects[0].clone();
    let timeline = Timeline {
        format: TimelineVersion::V02,
        nodes: (0..3)
            .map(|i| TimingNode {
                id: id(&format!("leaf{i}")),
                restart: RestartMode::Never,
                start: TimeCondition::Never {}.into(),
                end_conditions: vec![],
                duration: time(1000),
                repeat_milli: 1000.into(),
                repeat_duration: None,
                fill: FillMode::Hold,
                time_transform: None,
                effect: Effect::Rotation {
                    composition: Default::default(),
                    target: target.clone(),
                    from: i * 100,
                    to: (i + 1) * 100,
                },
            })
            .collect(),
        tree: Some(TimingTree {
            roots: vec![id("main")],
            containers: vec![TimingContainer {
                time_transform: None,
                presentation: None,
                id: id("main"),
                restart: RestartMode::Never,
                kind: ContainerKind::Sequence,
                start: TimeCondition::At { offset: time(0) }.into(),
                end_conditions: vec![],
                duration: ContainerDuration::Indefinite,
                fill: FillMode::Hold,
                children: (0..3).map(|i| id(&format!("leaf{i}"))).collect(),
                navigation: Some(SequenceNavigation {
                    concurrent,
                    next_action,
                    previous_action: PreviousAction::SkipTimed,
                    next_conditions: vec![TimeCondition::Navigation {
                        direction: NavigationDirection::Next,
                        target: None,
                        delay: time(0),
                    }],
                    previous_conditions: vec![TimeCondition::Navigation {
                        direction: NavigationDirection::Previous,
                        target: None,
                        delay: time(0),
                    }],
                }),
            }],
        }),
    };
    document.timelines.insert(slide, timeline.clone());
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
    let source = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
    let part = &source.slides[0].part;
    let xml = package
        .read_part(&PartName::new(part).unwrap(), 1 << 20, &|| false)
        .unwrap();
    (
        String::from_utf8(xml).unwrap(),
        source.surfaces[part]
            .objects
            .iter()
            .map(|o| o.native_id)
            .collect(),
        timeline,
    )
}
#[test]
fn exported_sequence_retains_navigation_policy_events_and_actual_playback() {
    for (concurrent, next_action) in [false, true]
        .into_iter()
        .flat_map(|c| [NextAction::None, NextAction::Seek].map(|a| (c, a)))
    {
        let (xml, known, original) = fixture_with_action(concurrent, next_action);
        assert!(xml.contains(if next_action == NextAction::Seek {
            "nextAc=\"seek\""
        } else {
            "nextAc=\"none\""
        }));
        assert!(xml.contains("prevAc=\"skipTimed\""));
        assert!(xml.contains("<p:nextCondLst>"));
        let native = read(xml.as_bytes(), &known).unwrap().unwrap();
        assert_eq!(
            native.timeline.tree.as_ref().unwrap().containers[0].navigation,
            original.tree.as_ref().unwrap().containers[0].navigation
        );
        let a = TimelinePlan::compile(&original, TimelineLimits::default(), &|| false).unwrap();
        let b =
            TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
        let history = input(&[
            (100, NavigationDirection::Next),
            (400, NavigationDirection::Next),
            (700, NavigationDirection::Previous),
            (900, NavigationDirection::Next),
        ]);
        for at in [0, 100, 200, 399, 400, 600, 700, 800, 900, 1200, 2000, 3000] {
            let mut a = a
                .evaluate(&binding(), time(at), Some(&history), &|| false)
                .unwrap()
                .state;
            let mut b = b
                .evaluate(&binding(), time(at), Some(&history), &|| false)
                .unwrap()
                .state;
            assert_eq!(
                unrotated_values(&a, RotationBasis::Absolute),
                unrotated_values(&b, RotationBasis::Layout)
            );
            assert_eq!(a.sequences[0].position, b.sequences[0].position);
            for (a, b) in a
                .nodes
                .iter_mut()
                .chain(&mut a.containers)
                .zip(b.nodes.iter_mut().chain(&mut b.containers))
            {
                a.node = id("same");
                b.node = id("same");
                assert_eq!(a, b, "at {at}");
            }
        }
    }
}
#[test]
fn actual_wps_saved_scale_tree_preserves_scope_identities_and_known_motion() {
    let native = read(
        include_bytes!("fixtures/wps-by-only-timing.xml"),
        &(2..=7).collect(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(native.node_bindings.len(), 15);
    assert_eq!(native.timeline.tree.as_ref().unwrap().containers.len(), 9);
    assert!(
        native
            .timeline
            .nodes
            .iter()
            .all(|n| n.restart == RestartMode::Always)
    );
    let containers = &native.timeline.tree.as_ref().unwrap().containers;
    assert_eq!(containers[1].restart, RestartMode::Always);
    assert_eq!(containers[2].restart, RestartMode::Always);
    assert!(
        containers[1..]
            .iter()
            .all(|c| c.duration == ContainerDuration::Automatic)
    );
    let plan =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    let history = input(&[]);
    for (at, numerator, denominator) in [
        // ExactScale uses 1/1000 percent, as does the public document model.
        (0, "100000", "1"),
        (500, "150000", "1"),
        (1000, "200000", "1"),
        (5000, "200000", "1"),
    ] {
        let frame = plan
            .evaluate(&binding(), time(at), Some(&history), &|| false)
            .unwrap();
        assert_eq!(frame.state.scales.len(), 6);
        for value in frame.state.scales.values() {
            assert_eq!(
                value.x,
                ExactValue {
                    numerator: numerator.into(),
                    denominator: denominator.into()
                }
            );
            assert_eq!(value.x, value.y);
        }
    }
}

#[test]
fn native_sequence_defaults_and_condition_targets_remain_explicit_in_computation() {
    let (xml, known, _) = fixture(false);
    let shape = *known.first().unwrap();
    let changed = xml
        .replace(" concurrent=\"0\" nextAc=\"none\" prevAc=\"skipTimed\"", "")
        .replace("<p:sldTgt/>", &format!("<p:spTgt spid=\"{shape}\"/>"));
    let native = read(changed.as_bytes(), &known).unwrap().unwrap();
    let nav = native.timeline.tree.as_ref().unwrap().containers[0]
        .navigation
        .as_ref()
        .unwrap();
    assert!(!nav.concurrent);
    assert_eq!(nav.next_action, NextAction::None);
    assert_eq!(nav.previous_action, PreviousAction::None);
    let target = ObjectId::new(format!("sp.{shape}")).unwrap();
    assert_eq!(
        nav.next_conditions,
        vec![TimeCondition::Navigation {
            direction: NavigationDirection::Next,
            target: Some(target.clone()),
            delay: time(0)
        }]
    );
    let mut history = input(&[(100, NavigationDirection::Next)]);
    let plan =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    assert_eq!(
        plan.evaluate(&binding(), time(200), Some(&history), &|| false)
            .unwrap()
            .state
            .nodes[0]
            .phase,
        NodePhase::Waiting
    );
    history.events[0].event = InputEvent::Navigation {
        direction: NavigationDirection::Next,
        target: Some(target),
    };
    assert_eq!(
        plan.evaluate(&binding(), time(200), Some(&history), &|| false)
            .unwrap()
            .state
            .nodes[0]
            .phase,
        NodePhase::Active
    );
}

#[test]
fn unknown_navigation_and_reordered_or_duplicate_content_is_not_dropped() {
    let (xml, known, _) = fixture(false);
    for (from, to) in [
        ("concurrent=\"0\"", "concurrent=\"maybe\""),
        ("nextAc=\"none\"", "nextAc=\"unknown\""),
        ("prevAc=\"skipTimed\"", "prevAc=\"unknown\""),
        ("<p:seq", "<p:seq unsupported=\"1\""),
        ("</p:seq>", "<p:nextCondLst/></p:seq>"),
        ("<p:prevCondLst>", "<p:nextCondLst/><p:prevCondLst>"),
        ("<p:prevCondLst>", "<p:prevCondLst unsupported=\"1\">"),
        ("<p:sldTgt/>", "<p:spTgt spid=\"99999\"/>"),
        ("evt=\"onNext\"", "evt=\"unknown\""),
        ("</p:seq>", "<p:extLst/></p:seq>"),
    ] {
        let changed = xml.replacen(from, to, 1);
        assert_ne!(xml, changed);
        assert!(read(changed.as_bytes(), &known).is_err(), "{to}");
    }
    let empty = xml.replace(
        "<p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>",
        "",
    );
    assert_ne!(empty, xml);
    assert!(read(empty.as_bytes(), &known).is_err());
}

#[test]
fn retained_hierarchy_and_navigation_conditions_obey_budgets_and_cancellation() {
    let (xml, known, _) = fixture(false);
    let run = |limits| {
        read_slide_timing(
            xml.as_bytes(),
            &known,
            XmlLimits::default(),
            limits,
            &|| false,
        )
    };
    assert!(
        run(TimelineLimits {
            max_nodes: 4,
            max_conditions: 6,
            max_depth: 2,
            ..Default::default()
        })
        .is_ok()
    );
    for limits in [
        TimelineLimits {
            max_nodes: 3,
            ..Default::default()
        },
        TimelineLimits {
            max_conditions: 5,
            ..Default::default()
        },
        TimelineLimits {
            max_depth: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(run(limits), Err(PptxError::Limit(_))));
    }
    for cancel_at in [1, 10, 40] {
        let calls = std::cell::Cell::new(0);
        let check = || {
            calls.set(calls.get() + 1);
            calls.get() >= cancel_at
        };
        assert!(matches!(
            read_slide_timing(
                xml.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &check
            ),
            Err(PptxError::Cancelled) | Err(PptxError::Xml(mo_xml::XmlError::Cancelled))
        ));
    }
}
