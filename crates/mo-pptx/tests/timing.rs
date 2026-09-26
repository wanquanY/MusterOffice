mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{
    source::{SourceLimits, inspect_source},
    timing::*,
    *,
};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn fixture() -> (mo_presentation_model::Document, ExportDefaults, SlideId) {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let nodes = (0..4)
        .map(|i| TimingNode {
            id: TimingNodeId::new(format!("a{i}")).unwrap(),
            start: match i {
                0 => StartCondition::At {
                    offset: t(125, 1000),
                },
                1 => StartCondition::After {
                    node: TimingNodeId::new("a0").unwrap(),
                    event: NodeEvent::End,
                    delay: t(1, 4),
                },
                2 => StartCondition::Click {
                    target: Some(target.clone()),
                    delay: t(0, 1),
                },
                _ => StartCondition::Click {
                    target: None,
                    delay: t(1, 2),
                },
            },
            duration: t(2, 1),
            end_conditions: vec![],
            repeat_milli: 2500.into(),
            repeat_duration: None,
            time_transform: None,
            fill: if i == 1 {
                FillMode::Remove
            } else {
                FillMode::Freeze
            },
            effect: Effect::Rotation {
                target: target.clone(),
                from: -43200000,
                to: 64800000,
            },
        })
        .collect();
    doc.timelines.insert(
        slide.clone(),
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes,
        },
    );
    (doc, defaults, slide)
}
fn bytes() -> Vec<u8> {
    let (doc, defaults, _) = fixture();
    export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn xml() -> (String, BTreeSet<u32>) {
    let b = bytes();
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let part = &index.slides[0].part;
    (
        String::from_utf8(
            p.read_part(&PartName::new(part).unwrap(), 1024 * 1024, &|| false)
                .unwrap(),
        )
        .unwrap(),
        index.surfaces[part]
            .objects
            .iter()
            .map(|o| o.native_id)
            .collect(),
    )
}
#[test]
fn native_roundtrip_preserves_multiturn_behavior_and_explicit_bindings() {
    let (doc, defaults, slide) = fixture();
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(b, bytes());
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let request = SourceTimingQuery {
        expected_source_sha256: index.source_sha256.clone(),
        slide: index.slides[0].part.clone(),
    };
    let result = query(
        &p,
        &request,
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let native = result.native.unwrap();
    assert_eq!(native.timeline.nodes.len(), 4);
    assert_eq!(native.node_bindings.len(), 4);
    assert_eq!(native.object_bindings.len(), 1);
    assert_eq!(native.root_id, 1);
    for (actual, expected) in native
        .timeline
        .nodes
        .iter()
        .zip(&doc.timelines[&slide].nodes)
    {
        assert_eq!(
            actual.duration.compare_time(expected.duration),
            std::cmp::Ordering::Equal
        );
        assert_eq!(actual.repeat_milli, expected.repeat_milli);
        assert_eq!(actual.fill, expected.fill);
        let (
            Effect::Rotation { target, from, to },
            Effect::Rotation {
                target: original,
                from: f,
                to: t,
            },
        ) = (&actual.effect, &expected.effect);
        assert_eq!((from, to), (f, t));
        let shape = index.surfaces[&request.slide]
            .objects
            .iter()
            .find(|o| o.native_id == native.object_bindings[target])
            .unwrap();
        assert_eq!(shape.name, original.as_str());
    }
    assert!(
        matches!(&native.timeline.nodes[1].start,StartCondition::After{node,event:NodeEvent::End,..} if *node==native.timeline.nodes[0].id)
    );
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("test").unwrap(),
        revision: index.source_sha256.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let history = EventHistory {
        binding: binding.clone(),
        through: t(8, 1),
        events: vec![],
    };
    let original =
        TimelinePlan::compile(&doc.timelines[&slide], TimelineLimits::default(), &|| false)
            .unwrap()
            .evaluate(&binding, t(6, 1), Some(&history), &|| false)
            .unwrap();
    let decoded = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding, t(6, 1), Some(&history), &|| false)
        .unwrap();
    assert_eq!(
        original.state.rotations.values().collect::<Vec<_>>(),
        decoded.state.rotations.values().collect::<Vec<_>>()
    );
    let mut stale = request;
    stale.expected_source_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(
            &p,
            &stale,
            SourceLimits::default(),
            TimelineLimits::default(),
            &|| false
        ),
        Err(PptxError::SourceConflict(_))
    ));
}
#[test]
fn export_refuses_rounding_and_keeps_static_document_bytes_unchanged() {
    let (mut doc, defaults, slide) = fixture();
    doc.timelines.get_mut(&slide).unwrap().nodes[0].duration = t(1001, 30000);
    assert!(matches!(
        export(
            &doc,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Unsupported(_))
    ));
    let (doc, defaults) = support::input();
    let original = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut empty = doc.clone();
    empty.timelines.insert(
        doc.slide_order[0].clone(),
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![],
        },
    );
    assert_eq!(
        original,
        export(
            &empty,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false
        )
        .unwrap()
    );
    let p = package(&original);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let request = SourceTimingQuery {
        expected_source_sha256: index.source_sha256,
        slide: index.slides[0].part.clone(),
    };
    assert!(
        query(
            &p,
            &request,
            SourceLimits::default(),
            TimelineLimits::default(),
            &|| false
        )
        .unwrap()
        .native
        .is_none()
    );
}
#[test]
fn unmapped_semantics_and_invalid_references_are_never_flattened() {
    let (source, known) = xml();
    for changed in [
        source.replacen("<p:animRot", "<p:animRot autoRev=\"1\"", 1),
        source.replacen("fill=\"freeze\"", "fill=\"transition\"", 1),
        source.replacen("additive=\"repl\"", "additive=\"sum\"", 1),
        source.replacen("restart=\"never\"", "restart=\"always\"", 1),
        source.replacen("<p:animRot", "<p:animRot by=\"100\"", 1),
        source.replacen("<p:childTnLst>", "<p:childTnLst><p:seq/>", 1),
        source.replacen(
            "<p:attrName>r</p:attrName>",
            "<p:attrName>ppt_x</p:attrName>",
            1,
        ),
        source.replacen(
            "<p:cond delay=\"125\"/>",
            "<p:cond delay=\"125\"/><p:cond delay=\"250\"/>",
            1,
        ),
        source.replacen("<p:tn val=\"2\"/>", "<p:tn val=\"999\"/>", 1),
        source
            .replacen("<p:timing>", "<p:extLst><p:timing>", 1)
            .replacen("</p:timing>", "</p:timing></p:extLst>", 1),
    ] {
        assert_ne!(changed, source);
        assert!(
            read_slide_timing(
                changed.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    assert!(
        read_slide_timing(
            source.as_bytes(),
            &BTreeSet::new(),
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert!(matches!(
        read_slide_timing(
            source.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits {
                max_nodes: 0,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_) | PptxError::Xml(mo_xml::XmlError::Limit(_)))
    ));
    assert!(matches!(
        read_slide_timing(
            source.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| true
        ),
        Err(PptxError::Cancelled | PptxError::Xml(mo_xml::XmlError::Cancelled))
    ));
}

fn tree_fixture() -> (mo_presentation_model::Document, ExportDefaults, SlideId) {
    let (mut doc, defaults, slide) = fixture();
    let timing = doc.timelines.get_mut(&slide).unwrap();
    timing.format = TimelineVersion::V02;
    for node in &mut timing.nodes {
        node.start = StartCondition::At { offset: t(1, 4) };
        node.duration = t(1, 1);
        node.repeat_milli = 1000.into();
    }
    let root = TimingNodeId::new("scope").unwrap();
    let inner = TimingNodeId::new("seq").unwrap();
    timing.tree = Some(TimingTree {
        roots: vec![root.clone()],
        containers: vec![
            TimingContainer {
                id: root,
                kind: ContainerKind::Parallel,
                start: StartCondition::At { offset: t(1, 2) },
                end_conditions: vec![],
                duration: ContainerDuration::Fixed { duration: t(4, 1) },
                fill: FillMode::Hold,
                children: vec![inner.clone(), timing.nodes[3].id.clone()],
            },
            TimingContainer {
                id: inner,
                kind: ContainerKind::Sequence,
                start: StartCondition::At { offset: t(0, 1) },
                end_conditions: vec![],
                duration: ContainerDuration::Automatic,
                fill: FillMode::Freeze,
                children: timing.nodes[..3].iter().map(|n| n.id.clone()).collect(),
            },
        ],
    });
    (doc, defaults, slide)
}
#[test]
fn native_tree_preserves_scoped_intervals_and_fill_at_boundaries() {
    let (doc, defaults, slide) = tree_fixture();
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let package = package(&b);
    let index = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
    let native = query(
        &package,
        &SourceTimingQuery {
            expected_source_sha256: index.source_sha256.clone(),
            slide: index.slides[0].part.clone(),
        },
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .native
    .unwrap();
    assert_eq!(native.timeline.format, TimelineVersion::V02);
    assert_eq!(native.node_bindings.len(), 6);
    let actual_tree = native.timeline.tree.as_ref().unwrap();
    assert_eq!(actual_tree.containers.len(), 2);
    assert_eq!(actual_tree.containers[1].kind, ContainerKind::Sequence);
    assert_eq!(
        actual_tree.containers[1].duration,
        ContainerDuration::Automatic
    );
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("native-tree").unwrap(),
        revision: index.source_sha256,
        generation: PlaybackGeneration::new(1),
    };
    let original =
        TimelinePlan::compile(&doc.timelines[&slide], TimelineLimits::default(), &|| false)
            .unwrap();
    let decoded =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    for n in [0, 1, 2, 3, 6, 7, 8, 11, 12, 16, 17, 18, 40] {
        let a = original
            .evaluate(&binding, t(n, 4), None, &|| false)
            .unwrap();
        let b = decoded
            .evaluate(&binding, t(n, 4), None, &|| false)
            .unwrap();
        assert_eq!(
            a.state.rotations.values().collect::<Vec<_>>(),
            b.state.rotations.values().collect::<Vec<_>>()
        );
        let strip = |f: &NodeFrame| {
            (
                f.phase,
                f.start.clone(),
                f.end.clone(),
                f.iteration.clone(),
                f.progress.clone(),
            )
        };
        assert_eq!(
            a.state.nodes.iter().map(strip).collect::<Vec<_>>(),
            b.state.nodes.iter().map(strip).collect::<Vec<_>>()
        );
        assert_eq!(
            a.state.containers.iter().map(strip).collect::<Vec<_>>(),
            b.state.containers.iter().map(strip).collect::<Vec<_>>()
        );
    }
    let xml = String::from_utf8(
        package
            .read_part(
                &PartName::new(&index.slides[0].part).unwrap(),
                1024 * 1024,
                &|| false,
            )
            .unwrap(),
    )
    .unwrap();
    let known = native.object_bindings.values().copied().collect();
    for changed in [
        xml.replacen("<p:seq>", "<p:seq concurrent=\"1\">", 1),
        xml.replacen("<p:seq>", "<p:seq nextAc=\"seek\">", 1),
        xml.replacen("<p:seq>", "<p:seq prevAc=\"skipTimed\">", 1),
        xml.replacen("<p:endSync evt=\"end\"", "<p:endSync evt=\"begin\"", 1),
        xml.replacen("<p:rtn val=\"all\"/>", "<p:rtn val=\"first\"/>", 1),
    ] {
        assert_ne!(changed, xml);
        assert!(
            read_slide_timing(
                changed.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    assert!(
        read_slide_timing(
            xml.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits {
                max_depth: 2,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
}
#[test]
fn empty_native_containers_remain_explicit_and_inexact_durations_refuse_export() {
    let (mut doc, defaults, slide) = tree_fixture();
    let timing = doc.timelines.get_mut(&slide).unwrap();
    timing.nodes.clear();
    let tree = timing.tree.as_mut().unwrap();
    tree.containers.truncate(1);
    tree.containers[0].children.clear();
    tree.containers[0].duration = ContainerDuration::Indefinite;
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let native = query(
        &p,
        &SourceTimingQuery {
            expected_source_sha256: index.source_sha256,
            slide: index.slides[0].part.clone(),
        },
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .native
    .unwrap();
    assert_eq!(native.timeline.node_count(), 1);
    assert_eq!(
        native.timeline.tree.unwrap().containers[0].duration,
        ContainerDuration::Indefinite
    );
    doc.timelines
        .get_mut(&slide)
        .unwrap()
        .tree
        .as_mut()
        .unwrap()
        .containers[0]
        .duration = ContainerDuration::Fixed { duration: t(1, 3) };
    assert!(matches!(
        export(
            &doc,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Unsupported(_))
    ));
}

#[test]
fn transformed_behavior_roundtrip_preserves_native_clock_and_sampled_values() {
    let (mut doc, defaults, slide) = tree_fixture();
    let timeline = doc.timelines.get_mut(&slide).unwrap();
    for (i, node) in timeline.nodes.iter_mut().enumerate() {
        node.time_transform = Some(TimeTransform {
            speed_milli_percent: if i % 2 == 0 { -125000 } else { 200000 },
            auto_reverse: i != 3,
            acceleration_milli_percent: 25000,
            deceleration_milli_percent: 50000,
        });
        node.repeat_milli = 2500.into();
    }
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let native = query(
        &p,
        &SourceTimingQuery {
            expected_source_sha256: index.source_sha256.clone(),
            slide: index.slides[0].part.clone(),
        },
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .native
    .unwrap();
    for (a, b) in doc.timelines[&slide]
        .nodes
        .iter()
        .zip(&native.timeline.nodes)
    {
        assert_eq!(a.time_transform, b.time_transform);
    }
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("time-transform").unwrap(),
        revision: index.source_sha256,
        generation: PlaybackGeneration::new(1),
    };
    let a = TimelinePlan::compile(&doc.timelines[&slide], TimelineLimits::default(), &|| false)
        .unwrap();
    let b = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    for at in [
        t(0, 1),
        t(1, 2),
        t(1, 1),
        t(5, 4),
        t(7, 4),
        t(2, 1),
        t(5, 2),
        t(3, 1),
        t(4, 1),
        t(5, 1),
        t(10, 1),
    ] {
        let x = a.evaluate(&binding, at, None, &|| false).unwrap();
        let y = b.evaluate(&binding, at, None, &|| false).unwrap();
        assert_eq!(
            x.state.rotations.values().collect::<Vec<_>>(),
            y.state.rotations.values().collect::<Vec<_>>()
        );
        for (x, y) in x.state.nodes.iter().zip(&y.state.nodes) {
            assert_eq!(
                (x.phase, &x.start, &x.end, &x.iteration, &x.progress),
                (y.phase, &y.start, &y.end, &y.iteration, &y.progress)
            );
        }
    }
}
#[test]
fn native_time_transform_percentages_are_exact_and_malformed_or_lossy_values_refuse_projection() {
    let (source, known) = xml();
    let change = |attributes: &str| {
        source.replacen("dur=\"2000\"", &format!("dur=\"2000\" {attributes}"), 1)
    };
    for (attributes, expected) in [
        (
            "spd=\"-125.0000%\" autoRev=\"true\" accel=\"25.00%\" decel=\"50%\"",
            TimeTransform {
                speed_milli_percent: -125000,
                auto_reverse: true,
                acceleration_milli_percent: 25000,
                deceleration_milli_percent: 50000,
            },
        ),
        (
            "spd=\"0.001%\"",
            TimeTransform {
                speed_milli_percent: 1,
                ..Default::default()
            },
        ),
        (
            "spd=\"-2147483648\" autoRev=\"0\"",
            TimeTransform {
                speed_milli_percent: i32::MIN,
                ..Default::default()
            },
        ),
        ("autoRev=\"false\"", TimeTransform::default()),
    ] {
        let xml = change(attributes);
        assert_ne!(xml, source);
        let n = read_slide_timing(
            xml.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(n.timeline.nodes[0].time_transform, Some(expected));
    }
    for attributes in [
        "spd=\"0\"",
        "spd=\"0.0001%\"",
        "spd=\"2147483648\"",
        "spd=\"1e2%\"",
        "spd=\"+100%\"",
        "accel=\"-1\"",
        "accel=\"0.001%\"",
        "accel=\"100001\"",
        "accel=\"50001\" decel=\"50000\"",
        "autoRev=\"yes\"",
        "repeatDur=\"unknown\"",
    ] {
        let xml = change(attributes);
        assert_ne!(xml, source);
        assert!(
            read_slide_timing(
                xml.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false
            )
            .is_err(),
            "{attributes}"
        );
    }
}

#[test]
fn repeat_bounds_roundtrip_preserves_native_duration_count_and_sampled_values() {
    let (mut doc, defaults, slide) = tree_fixture();
    let timeline = doc.timelines.get_mut(&slide).unwrap();
    for (i, node) in timeline.nodes.iter_mut().enumerate() {
        node.repeat_milli = if i % 2 == 0 {
            RepeatCount::Indefinite
        } else {
            3500.into()
        };
        node.repeat_duration = Some(if i == 3 {
            RepeatDuration::Indefinite
        } else {
            RepeatDuration::Finite(t(2500 + i as i64 * 125, 1000))
        });
        node.time_transform = Some(TimeTransform {
            speed_milli_percent: -125000,
            auto_reverse: i % 2 == 0,
            acceleration_milli_percent: 25000,
            deceleration_milli_percent: 25000,
        });
    }
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let native = query(
        &p,
        &SourceTimingQuery {
            expected_source_sha256: index.source_sha256.clone(),
            slide: index.slides[0].part.clone(),
        },
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .native
    .unwrap();
    for (a, b) in doc.timelines[&slide]
        .nodes
        .iter()
        .zip(&native.timeline.nodes)
    {
        assert_eq!(a.repeat_milli, b.repeat_milli);
        assert_eq!(a.repeat_duration, b.repeat_duration);
        assert_eq!(a.time_transform, b.time_transform);
    }
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("time-transform").unwrap(),
        revision: index.source_sha256,
        generation: PlaybackGeneration::new(1),
    };
    let a = TimelinePlan::compile(&doc.timelines[&slide], TimelineLimits::default(), &|| false)
        .unwrap();
    let b = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    for at in [
        t(0, 1),
        t(1, 2),
        t(1, 1),
        t(5, 4),
        t(7, 4),
        t(2, 1),
        t(5, 2),
        t(3, 1),
        t(4, 1),
        t(5, 1),
        t(10, 1),
    ] {
        let x = a.evaluate(&binding, at, None, &|| false).unwrap();
        let y = b.evaluate(&binding, at, None, &|| false).unwrap();
        assert_eq!(
            x.state.rotations.values().collect::<Vec<_>>(),
            y.state.rotations.values().collect::<Vec<_>>()
        );
        for (x, y) in x.state.nodes.iter().zip(&y.state.nodes) {
            assert_eq!(
                (x.phase, &x.start, &x.end, &x.iteration, &x.progress),
                (y.phase, &y.start, &y.end, &y.iteration, &y.progress)
            );
        }
    }
}

#[test]
fn native_repeat_bounds_accept_exact_unsigned_values_and_explicit_indefinite() {
    let (source, known) = xml();
    let read = |text: &str| {
        read_slide_timing(
            text.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
    };
    for (count, duration, expected_count, expected_duration) in [
        (
            Some("indefinite"),
            Some("1250"),
            RepeatCount::Indefinite,
            Some(RepeatDuration::Finite(t(1250, 1000))),
        ),
        (
            Some(" +3500 "),
            Some(" indefinite "),
            3500.into(),
            Some(RepeatDuration::Indefinite),
        ),
        (
            Some("4294967295"),
            Some("4294967295"),
            u32::MAX.into(),
            Some(RepeatDuration::Finite(t(u32::MAX.into(), 1000))),
        ),
        (
            None,
            Some("4000"),
            1000.into(),
            Some(RepeatDuration::Finite(t(4000, 1000))),
        ),
        (
            Some("indefinite"),
            Some("0"),
            RepeatCount::Indefinite,
            Some(RepeatDuration::Finite(t(0, 1000))),
        ),
        (Some("indefinite"), None, RepeatCount::Indefinite, None),
    ] {
        let text = source.replacen(
            "repeatCount=\"2500\"",
            &format!(
                "{} {}",
                count
                    .map(|v| format!("repeatCount=\"{v}\""))
                    .unwrap_or_default(),
                duration
                    .map(|v| format!("repeatDur=\"{v}\""))
                    .unwrap_or_default()
            ),
            1,
        );
        let n = read(&text).unwrap().unwrap();
        assert_eq!(n.timeline.nodes[0].repeat_milli, expected_count);
        assert_eq!(n.timeline.nodes[0].repeat_duration, expected_duration);
    }
    for (name, value) in [
        ("repeatCount", "0"),
        ("repeatCount", "-1"),
        ("repeatCount", "4294967296"),
        ("repeatCount", "1.5"),
        ("repeatCount", "Indefinite"),
        ("repeatDur", "-1"),
        ("repeatDur", "1.5"),
        ("repeatDur", "4294967296"),
        ("repeatDur", "unknown"),
    ] {
        let text = source.replacen("repeatCount=\"2500\"", &format!("{name}=\"{value}\""), 1);
        assert!(read(&text).is_err(), "{name}={value}");
    }
    let (mut doc, defaults, slide) = tree_fixture();
    doc.timelines.get_mut(&slide).unwrap().nodes[0].repeat_duration =
        Some(RepeatDuration::Finite(t(1, 3)));
    assert!(matches!(
        export(
            &doc,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Unsupported(_))
    ));
}

#[test]
fn end_conditions_roundtrip_preserves_native_references_and_sampled_values() {
    let (mut doc, defaults, slide) = tree_fixture();
    let timeline = doc.timelines.get_mut(&slide).unwrap();
    for (i, node) in timeline.nodes.iter_mut().enumerate() {
        node.end_conditions = vec![
            TimeCondition::After {
                node: node.id.clone(),
                event: NodeEvent::Begin,
                delay: t(750 + i as i64 * 125, 1000),
            },
            TimeCondition::At { offset: t(100, 1) },
        ];
        node.repeat_milli = if i % 2 == 0 {
            RepeatCount::Indefinite
        } else {
            3500.into()
        };
        node.repeat_duration = Some(if i == 3 {
            RepeatDuration::Indefinite
        } else {
            RepeatDuration::Finite(t(2500 + i as i64 * 125, 1000))
        });
        node.time_transform = Some(TimeTransform {
            speed_milli_percent: -125000,
            auto_reverse: i % 2 == 0,
            acceleration_milli_percent: 25000,
            deceleration_milli_percent: 25000,
        });
    }
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let native = query(
        &p,
        &SourceTimingQuery {
            expected_source_sha256: index.source_sha256.clone(),
            slide: index.slides[0].part.clone(),
        },
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .native
    .unwrap();
    for (a, b) in doc.timelines[&slide]
        .nodes
        .iter()
        .zip(&native.timeline.nodes)
    {
        assert_eq!(a.end_conditions.len(), b.end_conditions.len());
        assert!(
            matches!(&b.end_conditions[0], TimeCondition::After {node,event:NodeEvent::Begin,..} if node==&b.id)
        );
        assert_eq!(a.repeat_milli, b.repeat_milli);
        assert_eq!(a.repeat_duration, b.repeat_duration);
        assert_eq!(a.time_transform, b.time_transform);
    }
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("time-transform").unwrap(),
        revision: index.source_sha256,
        generation: PlaybackGeneration::new(1),
    };
    let a = TimelinePlan::compile(&doc.timelines[&slide], TimelineLimits::default(), &|| false)
        .unwrap();
    let b = TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    for at in [
        t(0, 1),
        t(1, 2),
        t(1, 1),
        t(5, 4),
        t(7, 4),
        t(2, 1),
        t(5, 2),
        t(3, 1),
        t(4, 1),
        t(5, 1),
        t(10, 1),
    ] {
        let x = a.evaluate(&binding, at, None, &|| false).unwrap();
        let y = b.evaluate(&binding, at, None, &|| false).unwrap();
        assert_eq!(
            x.state.rotations.values().collect::<Vec<_>>(),
            y.state.rotations.values().collect::<Vec<_>>()
        );
        for (x, y) in x.state.nodes.iter().zip(&y.state.nodes) {
            assert_eq!(
                (x.phase, &x.start, &x.end, &x.iteration, &x.progress),
                (y.phase, &y.start, &y.end, &y.iteration, &y.progress)
            );
        }
    }
}

#[test]
fn native_end_condition_lists_keep_order_and_reject_unmapped_or_invalid_contents() {
    let (source, known) = xml();
    let input = |list: &str| source.replacen("</p:stCondLst>", &format!("</p:stCondLst>{list}"), 1);
    let text = input(
        "<p:endCondLst><p:cond delay=\"5000\"/><p:cond evt=\"onClick\" delay=\"250\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond><p:cond evt=\"onBegin\" delay=\"500\"><p:tn val=\"2\"/></p:cond></p:endCondLst>",
    );
    let read = |text: &str, limits| {
        read_slide_timing(
            text.as_bytes(),
            &known,
            XmlLimits::default(),
            limits,
            &|| false,
        )
    };
    let n = read(&text, TimelineLimits::default()).unwrap().unwrap();
    let ends = &n.timeline.nodes[0].end_conditions;
    assert_eq!(ends.len(), 3);
    assert!(matches!(ends[0],TimeCondition::At {offset} if offset==t(5000,1000)));
    assert!(matches!(ends[1],TimeCondition::Click {target:None,delay} if delay==t(250,1000)));
    assert!(
        matches!(&ends[2],TimeCondition::After {node,event:NodeEvent::Begin,..} if node==&n.timeline.nodes[0].id)
    );
    assert!(
        read(
            &text,
            TimelineLimits {
                max_conditions: 4,
                ..Default::default()
            }
        )
        .is_err()
    );
    for list in [
        "<p:endCondLst/>",
        "<p:endCondLst><p:cond delay=\"0\" unknown=\"1\"/></p:endCondLst>",
        "<p:endCondLst><p:cond evt=\"onStopAudio\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:endCondLst>",
        "<p:endCondLst><p:cond evt=\"onEnd\" delay=\"0\"><p:tn val=\"2\"/></p:cond></p:endCondLst>",
        "<p:endCondLst><p:cond evt=\"onBegin\" delay=\"0\"><p:tn val=\"99999\"/></p:cond></p:endCondLst>",
        "<p:endCondLst><p:cond delay=\"indefinite\"/></p:endCondLst>",
        "<p:endCondLst><p:cond delay=\"0\"/></p:endCondLst><p:endCondLst><p:cond delay=\"0\"/></p:endCondLst>",
    ] {
        assert!(
            read(&input(list), TimelineLimits::default()).is_err(),
            "{list}"
        );
    }
    let (mut doc, defaults, slide) = tree_fixture();
    let node = &mut doc.timelines.get_mut(&slide).unwrap().nodes[0];
    node.end_conditions = vec![TimeCondition::After {
        node: node.id.clone(),
        event: NodeEvent::Begin,
        delay: t(1, 3),
    }];
    assert!(matches!(
        export(
            &doc,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Unsupported(_))
    ));
}
#[test]
fn native_container_ends_keep_child_references_and_real_clipped_intervals() {
    let (mut doc, defaults, slide) = tree_fixture();
    let timeline = doc.timelines.get_mut(&slide).unwrap();
    let child = timeline.nodes[0].id.clone();
    let containers = &mut timeline.tree.as_mut().unwrap().containers;
    containers[0].duration = ContainerDuration::Indefinite;
    containers[0].end_conditions = vec![TimeCondition::After {
        node: child,
        event: NodeEvent::Begin,
        delay: t(1, 2),
    }];
    containers[1].end_conditions = vec![TimeCondition::At { offset: t(3, 1) }];
    let b = export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&b);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let part = &index.slides[0].part;
    let source = String::from_utf8(
        p.read_part(&PartName::new(part).unwrap(), 1024 * 1024, &|| false)
            .unwrap(),
    )
    .unwrap();
    assert!(source.contains("<p:endCondLst><p:cond evt=\"onBegin\" delay=\"500\"><p:tn val=\"2\"/></p:cond></p:endCondLst>"));
    let known = index.surfaces[part]
        .objects
        .iter()
        .map(|o| o.native_id)
        .collect();
    let imported = read_slide_timing(
        source.as_bytes(),
        &known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("native-container").unwrap(),
        revision: Digest::from_sha256([9; 32]),
        generation: PlaybackGeneration::new(1),
    };
    let original =
        TimelinePlan::compile(&doc.timelines[&slide], TimelineLimits::default(), &|| false)
            .unwrap();
    let restored =
        TimelinePlan::compile(&imported.timeline, TimelineLimits::default(), &|| false).unwrap();
    for at in [t(0, 1), t(3, 4), t(1, 1), t(5, 4), t(10, 1)] {
        let a = original.evaluate(&binding, at, None, &|| false).unwrap();
        let b = restored.evaluate(&binding, at, None, &|| false).unwrap();
        for n in a.state.nodes.iter().chain(&a.state.containers) {
            let native = imported
                .node_bindings
                .iter()
                .find(|(_, value)| {
                    **value
                        == if n.node.as_str() == "scope" {
                            6
                        } else if n.node.as_str() == "seq" {
                            7
                        } else {
                            n.node.as_str()[1..].parse::<u32>().unwrap() + 2
                        }
                })
                .unwrap()
                .0;
            let m = b
                .state
                .nodes
                .iter()
                .chain(&b.state.containers)
                .find(|m| &m.node == native)
                .unwrap();
            assert_eq!(
                (n.phase, &n.start, &n.end, &n.progress),
                (m.phase, &m.start, &m.end, &m.progress)
            );
        }
    }
    for bad in [
        "<p:endCondLst/>",
        "<p:endCondLst><p:cond evt=\"onStopAudio\" delay=\"0\"/></p:endCondLst>",
        "<p:endCondLst><p:cond delay=\"indefinite\"/></p:endCondLst>",
    ] {
        let start = source.find("<p:endCondLst>").unwrap();
        let end =
            start + source[start..].find("</p:endCondLst>").unwrap() + "</p:endCondLst>".len();
        let mut changed = source.clone();
        changed.replace_range(start..end, bad);
        assert!(
            read_slide_timing(
                changed.as_bytes(),
                &known,
                XmlLimits::default(),
                TimelineLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    doc.timelines
        .get_mut(&slide)
        .unwrap()
        .tree
        .as_mut()
        .unwrap()
        .containers[0]
        .end_conditions = vec![TimeCondition::At { offset: t(1, 3) }];
    assert!(
        export(
            &doc,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false
        )
        .is_err()
    );
}
