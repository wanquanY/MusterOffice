mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{source::*, timing::*, *};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;

fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn read(xml: &str) -> Result<Option<NativeTimeline>, PptxError> {
    read_slide_timing(
        xml.as_bytes(),
        &BTreeSet::from([42]),
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
}
fn xml() -> String {
    format!(
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst>{}</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#,
        r#"<p:set><p:cBhvr><p:cTn id="2" dur="1000" fill="hold"><p:stCondLst><p:cond delay="500"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="42"/></p:tgtEl><p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val="visible"/></p:to></p:set>"#
    )
}
#[test]
fn native_set_requires_explicit_supported_property_and_discrete_value() {
    let x = xml();
    let n = read(&x).unwrap().unwrap();
    assert!(matches!(
        n.timeline.nodes[0].effect,
        Effect::SetVisibility {
            value: Visibility::Visible,
            ..
        }
    ));
    assert_eq!(n.timeline.nodes[0].restart, RestartMode::Always);
    for (from, to) in [
        ("style.visibility", "style.opacity"),
        ("val=\"visible\"", "val=\"collapse\""),
        ("<p:strVal val=\"visible\"/>", "<p:boolVal val=\"true\"/>"),
        (
            "<p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst>",
            "",
        ),
        ("<p:to><p:strVal val=\"visible\"/></p:to>", ""),
        ("<p:cBhvr>", "<p:cBhvr additive=\"sum\">"),
        ("<p:set>", "<p:set unsupported=\"1\">"),
        (
            "<p:strVal val=\"visible\"/>",
            "<p:strVal val=\"visible\"><p:extra/></p:strVal>",
        ),
    ] {
        assert!(read(&x.replace(from, to)).is_err(), "{from} -> {to}");
    }
    assert!(read(&x.replace("visible", "hidden")).is_ok());
}
#[test]
fn generic_assignment_does_not_invent_entrance_preset_semantics() {
    let x = xml();
    let body = x[x.find("<p:set>").unwrap()..x.find("</p:set>").unwrap() + 8].to_string();
    let wrapper = format!(
        r#"<p:par><p:cTn id="3" fill="hold" presetID="1" presetClass="entr" presetSubtype="0" nodeType="clickEffect"><p:childTnLst>{body}</p:childTnLst></p:cTn></p:par>"#
    );
    let entrance = read(&x.replace(&body, &wrapper)).unwrap().unwrap();
    assert!(matches!(
        entrance.timeline.tree.as_ref().unwrap().containers[0].presentation,
        Some(PresentationRole::Effect {
            preset: PresentationPreset::Appear,
            ..
        })
    ));
    let plain = format!(
        r#"<p:par><p:cTn id="3" fill="hold"><p:childTnLst>{body}</p:childTnLst></p:cTn></p:par>"#
    );
    assert!(
        read(&x.replace(&body, &plain))
            .unwrap()
            .unwrap()
            .timeline
            .tree
            .is_some()
    );
}
#[test]
fn editable_flat_and_tree_exports_roundtrip_exact_property_lifetimes() {
    for tree in [false, true] {
        let (mut doc, defaults) = support::input();
        let slide = doc.slide_order[0].clone();
        let target = doc.slides[&slide].objects[0].clone();
        let nodes = [Visibility::Hidden, Visibility::Visible]
            .into_iter()
            .enumerate()
            .map(|(i, value)| TimingNode {
                id: TimingNodeId::new(format!("set{i}")).unwrap(),
                restart: RestartMode::Never,
                start: TimeCondition::At {
                    offset: time(i as i64 * 500),
                }
                .into(),
                duration: time(1000),
                repeat_milli: 1000.into(),
                repeat_duration: None,
                end_conditions: vec![],
                time_transform: None,
                fill: if i == 0 {
                    FillMode::Hold
                } else {
                    FillMode::Remove
                },
                effect: Effect::SetVisibility {
                    target: target.clone(),
                    value,
                },
            })
            .collect::<Vec<_>>();
        let parent = TimingContainer {
            time_transform: None,
            presentation: None,
            id: TimingNodeId::new("parent").unwrap(),
            restart: RestartMode::Never,
            kind: ContainerKind::Parallel,
            navigation: None,
            start: TimeCondition::At { offset: time(0) }.into(),
            end_conditions: vec![],
            duration: ContainerDuration::Indefinite,
            fill: FillMode::Hold,
            children: nodes.iter().map(|n| n.id.clone()).collect(),
        };
        let timeline = Timeline {
            format: if tree {
                TimelineVersion::V02
            } else {
                TimelineVersion::V01
            },
            nodes,
            tree: tree.then(|| TimingTree {
                roots: vec![parent.id.clone()],
                containers: vec![parent],
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
        let p = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
        let part = &index.slides[0].part;
        let x = String::from_utf8(
            p.read_part(&PartName::new(part).unwrap(), 1 << 20, &|| false)
                .unwrap(),
        )
        .unwrap();
        assert!(x.contains("<p:set>") && x.contains("style.visibility"));
        assert!(!x.contains("presetClass=\"entr\"") && !x.contains("presetClass=\"exit\""));
        let known = index.surfaces[part]
            .objects
            .iter()
            .map(|o| o.native_id)
            .collect();
        let native = read_slide_timing(
            x.as_bytes(),
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(native.timeline.tree.is_some(), tree);
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("roundtrip").unwrap(),
            revision: index.source_sha256,
            generation: PlaybackGeneration::new(1),
        };
        let a = TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).unwrap();
        let b =
            TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
        for ms in [0, 499, 500, 1000, 1499, 1500, 3000] {
            let a = a.evaluate(&binding, time(ms), None, &|| false).unwrap();
            let b = b.evaluate(&binding, time(ms), None, &|| false).unwrap();
            assert_eq!(
                a.state.visibility.values().collect::<Vec<_>>(),
                b.state.visibility.values().collect::<Vec<_>>()
            );
        }
    }
}
