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
fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn read(xml: &str, known: &BTreeSet<u32>) -> Timeline {
    read_slide_timing(
        xml.as_bytes(),
        known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap()
    .timeline
}
#[test]
fn authored_native_path_keeps_original_controls_preset_and_sampled_positions() {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let path = MotionPath {
        from: p("0", "0"),
        segments: vec![
            MotionSegment::Cubic {
                control1: p("0.000000000000000001", "0.2"),
                control2: p("0.3", "-0.1"),
                to: p("0.4", "0"),
            },
            MotionSegment::Line {
                to: p("0.4", "0.1"),
            },
            MotionSegment::Close,
        ],
    };
    let sequence = PresentationSequence {
        groups: vec![PresentationGroup {
            start: PresentationGroupStart::Automatic,
            batches: vec![PresentationBatch {
                delay: time(0),
                effects: vec![PresentationEffect {
                    id: TimingNodeId::new("curve").unwrap(),
                    delay: time(0),
                    duration: time(1000),
                    repeat_milli: 1000.into(),
                    repeat_duration: None,
                    fill: FillMode::Hold,
                    time_transform: None,
                    effect: Effect::MotionPath {
                        target: doc.slides[&slide].objects[0].clone(),
                        path: path.clone(),
                    },
                }],
            }],
        }],
    };
    let timeline = sequence
        .compile(TimelineLimits::default(), &|| false)
        .unwrap();
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
    assert!(xml.contains("path=\"M 0 0 C 0.000000000000000001 0.2 0.3 -0.1 0.4 0 L 0.4 0.1 Z E\""));
    assert!(xml.contains("presetID=\"0\" presetClass=\"path\""));
    let known = index.surfaces[part]
        .objects
        .iter()
        .map(|o| o.native_id)
        .collect();
    let native = read(&xml, &known);
    let Effect::MotionPath { path: actual, .. } = &native.nodes[0].effect else {
        panic!("native path")
    };
    assert_eq!(actual, &path);
    let a = TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).unwrap();
    let b = TimelinePlan::compile(&native, TimelineLimits::default(), &|| false).unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("curve").unwrap(),
        revision: Digest::from_sha256([2; 32]),
        generation: PlaybackGeneration::new(1),
    };
    let history = EventHistory {
        binding: binding.clone(),
        through: time(10000),
        events: vec![],
    };
    for ms in [0, 1, 125, 333, 500, 750, 999, 1000, 3000, 250] {
        let a = a
            .evaluate(&binding, time(ms), Some(&history), &|| false)
            .unwrap();
        let b = b
            .evaluate(&binding, time(ms), Some(&history), &|| false)
            .unwrap();
        assert_eq!(a.state.profile, PACED_MOTION_FRAME_PROFILE);
        assert_eq!(
            a.state.motion.values().collect::<Vec<_>>(),
            b.state.motion.values().collect::<Vec<_>>()
        );
    }
}
#[test]
fn independent_relative_native_curve_and_close_use_exact_coordinate_origins() {
    let xml = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:timing><p:tnLst><p:par><p:cTn id="2" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst><p:animMotion origin="layout" path="m 0.1 0.2 c 0 0 0.1 -0.2 0.2 0 l 0 0.1 z E ignored"><p:cBhvr><p:cTn id="1" dur="1000" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="10"/></p:tgtEl></p:cBhvr></p:animMotion></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#;
    let timeline = read(xml, &BTreeSet::from([10]));
    let Effect::MotionPath { path, .. } = &timeline.nodes[0].effect else {
        panic!("native path")
    };
    assert_eq!(
        path,
        &MotionPath {
            from: p("0.1", "0.2"),
            segments: vec![
                MotionSegment::Cubic {
                    control1: p("0.1", "0.2"),
                    control2: p("0.2", "0"),
                    to: p("0.3", "0.2")
                },
                MotionSegment::Line {
                    to: p("0.3", "0.3")
                },
                MotionSegment::Close
            ]
        }
    );
}
