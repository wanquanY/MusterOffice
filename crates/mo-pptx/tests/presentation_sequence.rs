mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{
    source::{SourceLimits, inspect_source},
    timing::read_slide_timing,
    *,
};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;

#[test]
fn compiled_editorial_groups_survive_native_export_with_same_frames_and_roles() {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    let zero = RationalTime::new(0, 1).unwrap();
    let one = RationalTime::new(1, 1).unwrap();
    let sequence = PresentationSequence {
        groups: vec![PresentationGroup {
            start: PresentationGroupStart::Automatic,
            batches: (0..2)
                .map(|i| PresentationBatch {
                    delay: zero,
                    effects: vec![PresentationEffect {
                        id: TimingNodeId::new(format!("effect{i}")).unwrap(),
                        delay: zero,
                        duration: one,
                        repeat_milli: 1000.into(),
                        repeat_duration: None,
                        time_transform: None,
                        fill: FillMode::Hold,
                        effect: Effect::Scale {
                            target: target.clone(),
                            from: ScaleValue {
                                x: 100000,
                                y: 100000,
                            },
                            to: ScaleValue {
                                x: 200000 + i * 100000,
                                y: 200000 + i * 100000,
                            },
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
    let xml = package
        .read_part(&PartName::new(part).unwrap(), 1024 * 1024, &|| false)
        .unwrap();
    let ids = index.surfaces[part]
        .objects
        .iter()
        .map(|o| o.native_id)
        .collect::<BTreeSet<_>>();
    let native = read_slide_timing(
        &xml,
        &ids,
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .unwrap();
    let roles = |t: &Timeline| {
        t.tree
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .filter_map(|c| c.presentation)
            .collect::<Vec<_>>()
    };
    let mut original_roles = roles(&timeline);
    let mut imported_roles = roles(&native.timeline);
    // Native preorder may differ from the compiler's storage order.
    original_roles.sort_by_key(|v| format!("{v:?}"));
    imported_roles.sort_by_key(|v| format!("{v:?}"));
    assert_eq!(original_roles, imported_roles);
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("sequence").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    };
    let history = EventHistory {
        binding: binding.clone(),
        through: RationalTime::new(10, 1).unwrap(),
        events: vec![],
    };
    let authored = TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).unwrap();
    let imported =
        TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false).unwrap();
    for ms in [0, 500, 999, 1000, 1500, 2000, 5000, 750] {
        let at = RationalTime::new(ms, 1000).unwrap();
        let a = authored
            .evaluate(&binding, at, Some(&history), &|| false)
            .unwrap();
        let b = imported
            .evaluate(&binding, at, Some(&history), &|| false)
            .unwrap();
        assert_eq!(
            a.state.scales.values().collect::<Vec<_>>(),
            b.state.scales.values().collect::<Vec<_>>(),
            "at {ms}"
        );
    }
    let text = String::from_utf8(xml).unwrap();
    assert!(text.contains("nodeType=\"afterEffect\""));
    assert!(!text.contains("evt=\"end\"><p:tn"));
}
