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
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn starts(times: &[i64]) -> StartCondition {
    StartCondition::AnyOf {
        conditions: times
            .iter()
            .map(|n| TimeCondition::At { offset: time(*n) })
            .collect(),
    }
}
fn fixture(tree: bool) -> (String, BTreeSet<u32>, Timeline) {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let nodes = vec![RestartMode::Always, RestartMode::WhenNotActive]
        .into_iter()
        .enumerate()
        .map(|(i, restart)| TimingNode {
            id: id(&format!("r{i}")),
            restart,
            start: starts(&[0, 1, 4]),
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
    let timeline = Timeline {
        format: if tree {
            TimelineVersion::V02
        } else {
            TimelineVersion::V01
        },
        nodes,
        tree: tree.then(|| TimingTree {
            roots: vec![id("p")],
            containers: vec![TimingContainer {
                time_transform: None,
                presentation: None,
                navigation: None,
                id: id("p"),
                restart: RestartMode::Always,
                kind: ContainerKind::Parallel,
                start: starts(&[0, 6]),
                duration: ContainerDuration::Fixed { duration: time(5) },
                fill: FillMode::Hold,
                end_conditions: vec![],
                children: vec![id("r0"), id("r1")],
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
fn read(xml: &str, known: &BTreeSet<u32>) -> Result<Option<NativeTimeline>, PptxError> {
    read_slide_timing(
        xml.as_bytes(),
        known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
}
#[test]
fn explicit_restart_modes_roundtrip_native_flat_and_parent_scoped_frames() {
    for tree in [false, true] {
        let (xml, known, author) = fixture(tree);
        assert!(xml.contains("restart=\"always\""));
        assert!(xml.contains("restart=\"whenNotActive\""));
        let native = read(&xml, &known).unwrap().unwrap().timeline;
        assert_eq!(
            native.nodes.iter().map(|n| n.restart).collect::<Vec<_>>(),
            vec![RestartMode::Always, RestartMode::WhenNotActive]
        );
        if tree {
            assert_eq!(
                native.tree.as_ref().unwrap().containers[0].restart,
                RestartMode::Always
            );
        }
        let a = TimelinePlan::compile(&author, TimelineLimits::default(), &|| false).unwrap();
        let b = TimelinePlan::compile(&native, TimelineLimits::default(), &|| false).unwrap();
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("restart-roundtrip").unwrap(),
            revision: Digest::from_sha256([1; 32]),
            generation: PlaybackGeneration::new(1),
        };
        for n in [0, 1, 2, 3, 4, 8, 10, 12, 14, 16, 18, 22, 8] {
            let at = RationalTime::new(n, 2).unwrap();
            let fa = a.evaluate(&binding, at, None, &|| false).unwrap().state;
            let fb = b.evaluate(&binding, at, None, &|| false).unwrap().state;
            assert_eq!(fa.nodes.len(), fb.nodes.len());
            assert_eq!(fa.containers.len(), fb.containers.len());
            assert_eq!(
                unrotated_values(&fa, RotationBasis::Absolute),
                unrotated_values(&fb, RotationBasis::Layout)
            );
            for (mut a, mut b) in fa
                .nodes
                .into_iter()
                .chain(fa.containers)
                .zip(fb.nodes.into_iter().chain(fb.containers))
            {
                a.node = id("same");
                b.node = id("same");
                assert_eq!(a, b, "at {n}/2 tree={tree}");
            }
        }
    }
}
#[test]
fn missing_native_restart_uses_office_always_and_unknown_values_remain_errors() {
    for tree in [false, true] {
        let (xml, known, _) = fixture(tree);
        let absent = xml.replace(" restart=\"always\"", "");
        assert_ne!(absent, xml);
        // Native omission has an Office default distinct from author-model
        // omission (Never). Both leaf and container plans retain that policy.
        let expected = read(&xml, &known).unwrap().unwrap();
        let actual = read(&absent, &known).unwrap().unwrap();
        assert_eq!(actual.timeline, expected.timeline);
        for value in ["default", "false", ""] {
            let changed = xml.replace("restart=\"always\"", &format!("restart=\"{value}\""));
            assert!(read(&changed, &known).is_err());
        }
    }
}
