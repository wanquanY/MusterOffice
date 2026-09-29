//! Owned six-rectangle WPS calibration: only the timing subtree is retained.
mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{source::*, timing::*, *};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;

const XML: &str = include_str!("fixtures/appear-slide.xml");
fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn read(xml: &str, known: &BTreeSet<u32>) -> Result<NativeTimeline, PptxError> {
    read_slide_timing(
        xml.as_bytes(),
        known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .map(|v| v.unwrap())
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("appearance").unwrap(),
        revision: Digest::from_sha256([9; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn history() -> EventHistory {
    EventHistory {
        binding: binding(),
        through: time(5000),
        events: [
            (100, NavigationDirection::Next),
            (500, NavigationDirection::Previous),
            (1000, NavigationDirection::Next),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (ms, direction))| PlaybackEvent {
            generation: binding().generation,
            sequence: i as u32 + 1,
            at: time(ms),
            event: InputEvent::Navigation {
                direction,
                target: None,
            },
        })
        .collect(),
    }
}
fn values(t: &Timeline) -> Vec<Option<Visibility>> {
    let plan = TimelinePlan::compile(t, TimelineLimits::default(), &|| false).unwrap();
    let mut sampler = TimelineSampler::new(plan.clone());
    [
        0, 99, 100, 101, 499, 500, 999, 1000, 1001, 4000, 0, 100, 500,
    ]
    .into_iter()
    .map(|ms| {
        let direct = plan
            .evaluate(&binding(), time(ms), Some(&history()), &|| false)
            .unwrap();
        let retained = sampler
            .evaluate(&binding(), time(ms), Some(&history()), &|| false)
            .unwrap();
        assert_eq!(direct.sha256, retained.sha256);
        direct.state.visibility.values().next().copied()
    })
    .collect()
}
#[test]
fn native_appear_has_initial_hidden_triggered_visible_and_navigation_reset() {
    let native = read(XML, &BTreeSet::from([2])).unwrap();
    assert_eq!(
        native.timeline.tree.as_ref().unwrap().containers[0].presentation,
        Some(PresentationRole::MainSequence)
    );
    assert!(
        native
            .timeline
            .tree
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .any(|c| c.presentation
                == Some(PresentationRole::Effect {
                    preset: PresentationPreset::Appear,
                    trigger: PresentationTrigger::Click
                }))
    );
    use Visibility::*;
    assert_eq!(
        values(&native.timeline),
        vec![
            Hidden, Hidden, Visible, Visible, Visible, Hidden, Hidden, Visible, Visible, Visible,
            Hidden, Visible, Hidden
        ]
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>()
    );
}
#[test]
fn native_main_sequence_previous_undoes_an_active_click_group() {
    let native = read(
        &XML.replace("dur=\"1\"", "dur=\"2000\""),
        &BTreeSet::from([2]),
    )
    .unwrap();
    let values = values(&native.timeline);
    assert_eq!(values[4], Some(Visibility::Visible)); // Before Previous.
    assert_eq!(values[5], Some(Visibility::Hidden)); // Still-active group undone.
    assert_eq!(values[6], Some(Visibility::Hidden)); // Wait for a new trigger.
    assert_eq!(values[7], Some(Visibility::Visible)); // Replay.
}
#[test]
fn native_disappear_retains_document_baseline_until_triggered() {
    let xml = XML
        .replace("presetClass=\"entr\"", "presetClass=\"exit\"")
        .replace("val=\"visible\"", "val=\"hidden\"");
    let native = read(&xml, &BTreeSet::from([2])).unwrap();
    use Visibility::Hidden;
    assert_eq!(
        values(&native.timeline),
        vec![
            None,
            None,
            Some(Hidden),
            Some(Hidden),
            Some(Hidden),
            None,
            None,
            Some(Hidden),
            Some(Hidden),
            Some(Hidden),
            None,
            Some(Hidden),
            None
        ]
    );
}
#[test]
fn native_appearance_export_preserves_preset_identity_and_playback() {
    for exit in [false, true] {
        let xml = if exit {
            XML.replace("presetClass=\"entr\"", "presetClass=\"exit\"")
                .replace("val=\"visible\"", "val=\"hidden\"")
        } else {
            XML.into()
        };
        let (mut doc, defaults) = support::input();
        let slide = doc.slide_order[0].clone();
        let target = doc.slides[&slide].objects[0].clone();
        let mut t = read(&xml, &BTreeSet::from([2])).unwrap().timeline;
        t.nodes[0].effect = Effect::SetVisibility {
            target,
            value: if exit {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
        };
        doc.timelines.insert(slide, t.clone());
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
        let xml = String::from_utf8(
            p.read_part(&PartName::new(part).unwrap(), 1 << 20, &|| false)
                .unwrap(),
        )
        .unwrap();
        assert!(xml.contains("nodeType=\"mainSeq\"") && xml.contains("nodeType=\"clickEffect\""));
        assert!(xml.contains(if exit {
            "presetClass=\"exit\""
        } else {
            "presetClass=\"entr\""
        }));
        let known = index.surfaces[part]
            .objects
            .iter()
            .map(|o| o.native_id)
            .collect();
        assert_eq!(values(&read(&xml, &known).unwrap().timeline), values(&t));
    }
}
#[test]
fn incomplete_or_mismatched_presets_and_observable_omitted_fill_are_rejected() {
    for (a, b) in [
        ("presetID=\"1\"", "presetID=\"2\""),
        ("presetSubtype=\"0\"", "presetSubtype=\"1\""),
        ("presetClass=\"entr\"", "presetClass=\"exit\""),
        ("val=\"visible\"", "val=\"hidden\""),
        (
            "dur=\"indefinite\" nodeType=\"mainSeq\"",
            "dur=\"500\" nodeType=\"mainSeq\"",
        ),
        ("id=\"4\" fill=\"hold\"", "id=\"4\""),
    ] {
        assert!(XML.contains(a));
        assert!(
            read(&XML.replace(a, b), &BTreeSet::from([2])).is_err(),
            "{a} -> {b}"
        );
    }
}
