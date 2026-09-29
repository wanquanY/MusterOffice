mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{source::*, timing::*, *};
use mo_timeline::*;
use mo_xml::XmlLimits;
use serde_json::json;

#[test]
fn native_container_speed_preserves_scope_delays_and_fixed_duration() {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let timeline: Timeline = serde_json::from_value(json!({
        "format":"musteroffice.timeline/0.2-draft",
        "nodes":[{"id":"line","start":{"kind":"at","offset":{"ticks":"1","timescale":1}},"duration":{"ticks":"6","timescale":1},"repeatMilli":1000,"fill":"hold","effect":{"kind":"motionLine","target":target,"from":{"x":"0","y":"0"},"to":{"x":"0.25","y":"0.5"}}}],
        "tree":{"roots":["group"],"containers":[{"id":"group","kind":"parallel","start":{"kind":"at","offset":{"ticks":"2","timescale":1}},"duration":{"kind":"fixed","duration":{"ticks":"4","timescale":1}},"fill":"hold","children":["line"],"timeTransform":{"speedMilliPercent":200000,"autoReverse":false,"accelerationMilliPercent":0,"decelerationMilliPercent":0}}]}
    })).unwrap();
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
    let source = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
    let part = &source.slides[0].part;
    let xml = package
        .read_part(&PartName::new(part).unwrap(), 1 << 20, &|| false)
        .unwrap();
    let text = std::str::from_utf8(&xml).unwrap();
    assert_eq!(text.matches("spd=\"200000\"").count(), 1);
    assert!(text.contains("dur=\"4000\""));
    assert!(text.contains("dur=\"6000\""));
    let known = source.surfaces[part]
        .objects
        .iter()
        .map(|o| o.native_id)
        .collect();
    let native = read_slide_timing(
        &xml,
        &known,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap();
    assert!(native.timeline.nodes[0].time_transform.is_none());
    assert_eq!(
        native
            .timeline
            .tree
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .filter(|c| c.time_transform.is_some())
            .count(),
        1
    );
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("native-rate").unwrap(),
        revision: Digest::from_sha256([23; 32]),
        generation: PlaybackGeneration::new(1),
    };
    for input in [&timeline, &native.timeline] {
        let plan = TimelinePlan::compile(input, TimelineLimits::default(), &|| false).unwrap();
        for (at, n, d) in [(3, "1", "6"), (4, "1", "2"), (9, "1", "2")] {
            let frame = plan
                .evaluate(&binding, RationalTime::new(at, 1).unwrap(), None, &|| false)
                .unwrap();
            assert_eq!(
                frame.state.nodes[0].start,
                Some(ExactValue {
                    numerator: "5".into(),
                    denominator: "2".into()
                })
            );
            assert_eq!(
                frame.state.nodes[0].end,
                Some(ExactValue {
                    numerator: "4".into(),
                    denominator: "1".into()
                })
            );
            assert_eq!(
                frame.state.nodes[0].progress,
                Some(ExactValue {
                    numerator: n.into(),
                    denominator: d.into()
                })
            );
        }
    }
}
