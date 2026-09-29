//! Handwritten PresentationML; none of these inputs come from our writer.
use mo_common::*;
use mo_pptx::{PptxError, timing::*};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;
mod support;
fn read(attrs: &str, mode: &str) -> Result<NativeTimeline, PptxError> {
    let xml = format!(
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst><p:animRot {attrs}><p:cBhvr {mode}><p:cTn id="2" dur="1000" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="10"/></p:tgtEl><p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#
    );
    read_slide_timing(
        xml.as_bytes(),
        &BTreeSet::from([10]),
        XmlLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .map(Option::unwrap)
}
#[test]
fn native_angle_grammar_keeps_by_composition_distinct_from_endpoints() {
    for (attrs, from, to, composition) in [
        (
            r#"from="-600000" to="1800000""#,
            -600000,
            1800000,
            RotationComposition::Layout,
        ),
        (
            r#"from="-600000" by="2400000""#,
            -600000,
            1800000,
            RotationComposition::Layout,
        ),
        (r#"to="1800000""#, 0, 1800000, RotationComposition::Layout),
        (r#"by="-43200000""#, 0, -43200000, RotationComposition::Add),
        (
            r#"from="-600000" to="1800000" by="1""#,
            -600000,
            1800000,
            RotationComposition::Layout,
        ),
        (
            r#"to="1800000" by="1""#,
            0,
            1800000,
            RotationComposition::Layout,
        ),
    ] {
        for mode in ["", r#"additive="base""#, r#"additive="repl""#] {
            let native = read(attrs, mode).unwrap();
            assert_eq!(
                native.timeline.nodes[0].effect,
                Effect::Rotation {
                    target: ObjectId::new("sp.10").unwrap(),
                    from,
                    to,
                    composition
                }
            );
            let plan =
                TimelinePlan::compile(&native.timeline, TimelineLimits::default(), &|| false)
                    .unwrap();
            let b = PlaybackBinding {
                session: PlaybackSessionId::new("native-rotation").unwrap(),
                revision: Digest::from_sha256([14; 32]),
                generation: PlaybackGeneration::new(1),
            };
            let f = plan
                .evaluate(&b, RationalTime::new(1, 2).unwrap(), None, &|| false)
                .unwrap();
            let v = &f.state.rotations[&ObjectId::new("sp.10").unwrap()];
            assert_eq!(v.basis, RotationBasis::Layout);
            assert_eq!(
                v.numerator,
                ((i64::from(from) + i64::from(to)) / 2).to_string()
            );
            assert_eq!(v.denominator, "1");
        }
    }
}
#[test]
fn invalid_or_unmapped_native_rotation_is_never_guessed() {
    for attrs in [
        "",
        r#"from="1""#,
        r#"by="2147483648""#,
        r#"from="2147483647" by="1""#,
        r#"to="1" by="bad""#,
        r#"from="nan" to="1""#,
    ] {
        assert!(read(attrs, "").is_err(), "{attrs}");
    }
    for mode in [
        r#"additive="sum""#,
        r#"additive="mult""#,
        r#"additive="none""#,
        r#"accumulate="always""#,
        r#"xfrmType="img""#,
    ] {
        assert!(
            matches!(read(r#"by="600000""#, mode), Err(PptxError::Unsupported(_))),
            "{mode}"
        );
    }
}
#[test]
fn absolute_author_export_converts_original_rotation_without_changing_the_document() {
    let (mut doc, defaults) = support::input();
    let slide = doc.slide_order[0].clone();
    let target = doc.slides[&slide].objects[0].clone();
    doc.objects
        .get_mut(&target)
        .unwrap()
        .transform
        .as_mut()
        .unwrap()
        .rotation = 1200000;
    let mut node = read(r#"from="0" to="1800000""#, "")
        .unwrap()
        .timeline
        .nodes
        .remove(0);
    node.effect = Effect::Rotation {
        target: target.clone(),
        from: 0,
        to: 1800000,
        composition: RotationComposition::Absolute,
    };
    doc.timelines.insert(
        slide,
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![node],
        },
    );
    let digest = doc.semantic_digest().unwrap();
    let bytes = mo_pptx::export(
        &doc,
        &defaults,
        &support::resources(),
        mo_pptx::PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(doc.semantic_digest().unwrap(), digest);
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
    let native = query(
        &package,
        &SourceTimingQuery {
            expected_source_sha256: index.source_sha256,
            slide: index.slides[0].part.clone(),
        },
        mo_pptx::source::SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
    .native
    .unwrap();
    assert!(matches!(
        native.timeline.nodes[0].effect,
        Effect::Rotation {
            from: -1200000,
            to: 600000,
            composition: RotationComposition::Layout,
            ..
        }
    ));
}
