//! Hand-authored native conditions, independent of the exporter's spelling.
mod support;
use mo_common::*;
use mo_pptx::{PptxError, timing::*};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;

fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn exact(n: i64, d: u32) -> Option<ExactValue> {
    let (mut a, mut b) = (n.unsigned_abs(), u64::from(d));
    while b != 0 {
        (a, b) = (b, a % b);
    }
    Some(ExactValue {
        numerator: (n / a as i64).to_string(),
        denominator: (u64::from(d) / a).to_string(),
    })
}
fn behavior(id: u32, target: u32, start: &str, ends: &str) -> String {
    format!(
        r#"<p:animRot from="0" to="120"><p:cBhvr additive="repl"><p:cTn id="{id}" dur="2000" restart="never" fill="hold"><p:stCondLst>{start}</p:stCondLst>{ends}</p:cTn><p:tgtEl><p:spTgt spid="{target}"/></p:tgtEl></p:cBhvr></p:animRot>"#
    )
}
fn fixture(condition: &str, ends: &str) -> String {
    let first = behavior(2, 10, r#"<p:cond delay="1000"/>"#, "");
    let second = behavior(3, 20, condition, ends);
    format!(
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst>{first}{second}</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#
    )
}
fn read(xml: &str) -> Result<NativeTimeline, PptxError> {
    read_slide_timing(
        xml.as_bytes(),
        &BTreeSet::from([10, 20]),
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .map(Option::unwrap)
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("native-node-events").unwrap(),
        revision: Digest::from_sha256([19; 32]),
        generation: PlaybackGeneration::new(1),
    }
}

#[test]
fn explicit_node_edges_fire_at_the_referenced_begin_or_end() {
    for (event, edge) in [
        ("begin", 1000),
        ("end", 3000),
        ("onBegin", 1000),
        ("onEnd", 3000),
    ] {
        for delay in [0, 250] {
            let condition = if delay == 0 {
                format!(r#"<p:cond evt="{event}"><p:tn val="2"/></p:cond>"#)
            } else {
                format!(r#"<p:cond evt="{event}" delay="250"><p:tn val="2"/></p:cond>"#)
            };
            let native = read(&fixture(&condition, "")).unwrap();
            let wire = serde_json::to_value(&native.timeline).unwrap();
            assert_eq!(wire["nodes"][1]["start"]["event"], event);
            let plan =
                TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false)
                    .unwrap();
            let start = edge + delay;
            // Include backwards evaluation after completion. Expected values
            // come from independent arithmetic, not another execution path.
            for at in [start - 1, start, start + 1000, start + 2000, start + 1] {
                let state = plan
                    .evaluate(&binding(), time(at), None, &|| false)
                    .unwrap()
                    .state;
                let node = &state.nodes[1];
                assert_eq!(node.start, exact(start, 1000), "{event}/{delay}/{at}");
                let rotation = state.rotations.get(&ObjectId::new("sp.20").unwrap());
                if at < start {
                    assert!(rotation.is_none());
                } else {
                    let n = (at - start).min(2000) * 120;
                    assert_eq!(rotation.map(ExactRotation::value), exact(n, 2000));
                }
            }
        }
    }
}

#[test]
fn node_event_identity_survives_native_read_author_export_and_read() {
    use mo_opc::{Package, PackageLimits, PartName};
    use mo_pptx::{
        PptxLimits, export,
        source::{SourceLimits, inspect_source},
    };
    for event in ["begin", "end", "onBegin", "onEnd"] {
        let native = read(&fixture(
            &format!(r#"<p:cond evt="{event}" delay="250"><p:tn val="2"/></p:cond>"#),
            "",
        ))
        .unwrap();
        let (mut document, defaults) = support::input();
        let slide = document.slide_order[0].clone();
        let object = document.slides[&slide].objects[0].clone();
        let mut timeline = native.timeline;
        for node in &mut timeline.nodes {
            if let Effect::Rotation { target, .. } = &mut node.effect {
                *target = object.clone();
            }
        }
        document.timelines.insert(slide, timeline);
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
            .read_part(&PartName::new(part).unwrap(), 1024 * 1024, &|| false)
            .unwrap();
        let known = source.surfaces[part]
            .objects
            .iter()
            .map(|object| object.native_id)
            .collect();
        let roundtrip = read_slide_timing(
            &xml,
            &known,
            XmlLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            serde_json::to_value(&roundtrip.timeline).unwrap()["nodes"][1]["start"]["event"],
            event
        );
        assert!(
            String::from_utf8(xml)
                .unwrap()
                .contains(&format!(r#"evt="{event}" delay="250""#))
        );
    }
}

#[test]
fn condition_defaults_preserve_absolute_offsets_and_explicit_never() {
    for condition in ["<p:cond/>", r#"<p:cond delay="0"/>"#] {
        let native = read(&fixture(condition, "")).unwrap();
        assert_eq!(
            native.timeline.nodes[1].start,
            TimeCondition::At { offset: time(0) }.into()
        );
    }
    let native = read(&fixture(r#"<p:cond delay="125"/>"#, "")).unwrap();
    assert_eq!(
        native.timeline.nodes[1].start,
        TimeCondition::At { offset: time(125) }.into()
    );
    let native = read(&fixture(r#"<p:cond delay="indefinite"/>"#, "")).unwrap();
    assert_eq!(
        native.timeline.nodes[1].start,
        TimeCondition::Never {}.into()
    );
}

#[test]
fn end_conditions_share_defaults_and_node_edges() {
    let xml = fixture(
        "<p:cond/>",
        r#"<p:endCondLst><p:cond evt="begin"><p:tn val="2"/></p:cond></p:endCondLst>"#,
    );
    let native = read(&xml).unwrap();
    let plan =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    let state = plan
        .evaluate(&binding(), time(1500), None, &|| false)
        .unwrap()
        .state;
    assert_eq!(state.nodes[1].end, exact(1, 1));
    assert_eq!(
        state.rotations[&ObjectId::new("sp.20").unwrap()].value(),
        exact(60, 1).unwrap()
    );
}

#[test]
fn omitted_click_delay_still_waits_for_the_correct_target() {
    let xml = fixture(
        r#"<p:cond evt="onClick"><p:tgtEl><p:spTgt spid="20"/></p:tgtEl></p:cond>"#,
        "",
    );
    let native = read(&xml).unwrap();
    let plan =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    let binding = binding();
    let history = EventHistory {
        binding: binding.clone(),
        through: time(5000),
        events: [10, 20]
            .into_iter()
            .enumerate()
            .map(|(index, target)| PlaybackEvent {
                generation: binding.generation,
                sequence: index as u32 + 1,
                at: time((index as i64 + 1) * 1000),
                event: InputEvent::Click {
                    target: Some(ObjectId::new(format!("sp.{target}")).unwrap()),
                },
            })
            .collect(),
    };
    let state = plan
        .evaluate(&binding, time(2500), Some(&history), &|| false)
        .unwrap()
        .state;
    assert_eq!(state.nodes[1].start, exact(2, 1));
    assert_eq!(
        state.rotations[&ObjectId::new("sp.20").unwrap()].value(),
        exact(30, 1).unwrap()
    );
}

#[test]
fn malformed_or_unmapped_targets_are_never_inferred_from_neighbors() {
    for condition in [
        r#"<p:cond evt="none"/>"#,
        r#"<p:cond evt="none" delay="indefinite"/>"#,
        r#"<p:cond evt="begin"/>"#,
        r#"<p:cond evt="end" delay="1000"/>"#,
        r#"<p:cond evt="onEnd" delay="1000"/>"#,
        r#"<p:cond evt="end"><p:tn val="999"/></p:cond>"#,
        r#"<p:cond evt="end"><p:rtn val="last"/></p:cond>"#,
        r#"<p:cond evt="onEnd"><p:tgtEl><p:spTgt spid="10"/></p:tgtEl></p:cond>"#,
        r#"<p:cond evt="begin" delay="indefinite"><p:tn val="2"/></p:cond>"#,
        r#"<p:cond evt="none"><p:tn val="2"/></p:cond>"#,
        r#"<p:cond evt="none" delay="indefinite"><p:tn val="2"/></p:cond>"#,
        r#"<p:cond delay=""/>"#,
        r#"<p:cond delay="-1"/>"#,
        r#"<p:cond delay="4294967296"/>"#,
        r#"<p:cond evt="onClick"/>"#,
    ] {
        assert!(read(&fixture(condition, "")).is_err(), "{condition}");
    }
}
