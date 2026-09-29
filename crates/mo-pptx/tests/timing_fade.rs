//! Timing subtree authored in WPS using this project's owned six-rectangle deck.
use mo_common::*;
use mo_pptx::timing::*;
use mo_timeline::*;
use std::collections::BTreeSet;
const XML: &str = include_str!("fixtures/fade-slide.xml");
fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn read(xml: &str) -> Result<NativeTimeline, mo_pptx::PptxError> {
    read_slide_timing(
        xml.as_bytes(),
        &BTreeSet::from([2, 3]),
        mo_xml::XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .map(|v| v.unwrap())
}
#[test]
fn wps_compound_fades_lift_with_distinct_clocks_and_editable_presets() {
    let native = read(XML).unwrap();
    assert_eq!(native.timeline.nodes.len(), 4);
    let presets: Vec<_> = native
        .timeline
        .tree
        .as_ref()
        .unwrap()
        .containers
        .iter()
        .filter_map(|c| c.presentation)
        .collect();
    assert!(presets.contains(&PresentationRole::Effect {
        preset: PresentationPreset::FadeIn,
        trigger: PresentationTrigger::Click
    }));
    assert!(presets.contains(&PresentationRole::Effect {
        preset: PresentationPreset::FadeOut,
        trigger: PresentationTrigger::Click
    }));
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("wps-fade").unwrap(),
        revision: Digest::from_sha256([7; 32]),
        generation: PlaybackGeneration::new(1),
    };
    let history = EventHistory {
        binding: binding.clone(),
        through: t(5000),
        events: [
            (100, NavigationDirection::Next),
            (1000, NavigationDirection::Next),
            (2000, NavigationDirection::Previous),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (ms, direction))| PlaybackEvent {
            generation: binding.generation,
            sequence: i as u32 + 1,
            at: t(ms),
            event: InputEvent::Navigation {
                direction,
                target: None,
            },
        })
        .collect(),
    };
    let plan =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    let mut sampler = TimelineSampler::new(plan.clone());
    for ms in [0, 100, 350, 600, 1000, 1250, 1498, 1499, 1500, 2000, 350, 0] {
        let a = plan
            .evaluate(&binding, t(ms), Some(&history), &|| false)
            .unwrap();
        let b = sampler
            .evaluate(&binding, t(ms), Some(&history), &|| false)
            .unwrap();
        assert_eq!(a.sha256, b.sha256);
        let red = ObjectId::new("sp.2").unwrap();
        let green = ObjectId::new("sp.3").unwrap();
        if ms == 0 {
            assert_eq!(a.state.visibility[&red], Visibility::Hidden);
        }
        if ms == 350 || ms == 1250 {
            let id = if ms == 350 { &red } else { &green };
            assert_eq!(
                a.state.opacity[id],
                ExactValue {
                    numerator: "1".into(),
                    denominator: "2".into()
                }
            );
        }
        if ms == 1498 {
            assert!(!a.state.visibility.contains_key(&green));
        }
        if ms == 1499 || ms == 1500 {
            assert_eq!(a.state.visibility[&green], Visibility::Hidden);
        }
        if ms == 600 || ms == 1500 || ms == 2000 {
            assert!(a.state.opacity.is_empty());
        }
        if ms == 2000 {
            assert!(!a.state.visibility.contains_key(&green));
        }
    }
}
#[test]
fn unknown_filters_progress_additive_and_mismatched_compound_targets_fail_projection() {
    for bad in [
        XML.replace("filter=\"fade\"", "filter=\"blinds\""),
        XML.replace("transition=\"in\"", "transition=\"none\""),
        XML.replacen("<p:cBhvr>", "<p:cBhvr additive=\"sum\">", 1),
        XML.replacen("spid=\"2\"", "spid=\"3\"", 1),
        XML.replacen(
            "</p:animEffect>",
            "<p:progress><p:fltVal val=\"0.5\"/></p:progress></p:animEffect>",
            1,
        ),
    ] {
        assert!(read(&bad).is_err());
    }
}

mod support;
#[test]
fn authored_compound_fades_export_native_editable_behavior_and_preserve_frames() {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let sequence = PresentationSequence {
        groups: vec![PresentationGroup {
            start: PresentationGroupStart::Automatic,
            batches: [FadeTransition::In, FadeTransition::Out]
                .into_iter()
                .enumerate()
                .map(|(i, transition)| PresentationBatch {
                    delay: t(100),
                    effects: vec![PresentationEffect {
                        id: TimingNodeId::new(format!("fade{i}")).unwrap(),
                        delay: t(0),
                        duration: t(1000),
                        repeat_milli: 1000.into(),
                        repeat_duration: None,
                        fill: FillMode::Remove,
                        time_transform: None,
                        effect: Effect::Fade {
                            target: target.clone(),
                            transition,
                        },
                    }],
                })
                .collect(),
        }],
    };
    let timeline = sequence
        .compile(TimelineLimits::default(), &|| false)
        .unwrap();
    doc.timelines.insert(slide, timeline.clone());
    let bytes = mo_pptx::export(
        &doc,
        &defaults,
        &support::resources(),
        mo_pptx::PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let package = mo_opc::Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        mo_opc::PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = mo_pptx::source::inspect_source(
        &package,
        mo_pptx::source::SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    let part = &index.slides[0].part;
    let xml = package
        .read_part(&mo_opc::PartName::new(part).unwrap(), 1024 * 1024, &|| {
            false
        })
        .unwrap();
    let known = index.surfaces[part]
        .objects
        .iter()
        .map(|o| o.native_id)
        .collect();
    let imported = read_slide_timing(
        &xml,
        &known,
        mo_xml::XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("fade-export").unwrap(),
        revision: Digest::from_sha256([5; 32]),
        generation: PlaybackGeneration::new(1),
    };
    let history = EventHistory {
        binding: binding.clone(),
        through: t(5000),
        events: vec![],
    };
    let original = TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).unwrap();
    let native =
        TimelinePlan::compile(&imported.timeline, TimelineLimits::default(), &|| false).unwrap();
    for ms in [
        0, 100, 350, 600, 1100, 1200, 1700, 2198, 2199, 2200, 4500, 0,
    ] {
        let a = original
            .evaluate(&binding, t(ms), Some(&history), &|| false)
            .unwrap()
            .state;
        let b = native
            .evaluate(&binding, t(ms), Some(&history), &|| false)
            .unwrap()
            .state;
        assert_eq!(
            a.opacity.values().collect::<Vec<_>>(),
            b.opacity.values().collect::<Vec<_>>(),
            "opacity at {ms}"
        );
        assert_eq!(
            a.visibility.values().collect::<Vec<_>>(),
            b.visibility.values().collect::<Vec<_>>(),
            "visibility at {ms}"
        );
    }
    let xml = String::from_utf8(xml).unwrap();
    assert_eq!(xml.matches("<p:animEffect filter=\"fade\"").count(), 2);
    assert!(xml.contains("presetID=\"10\""));
}
