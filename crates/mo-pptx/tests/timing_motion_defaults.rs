//! Native fixtures independent of the writer. Offsets are from the original
//! layout, even while another path is active or held on the same object.
use mo_common::*;
use mo_pptx::{PptxError, timing::*};
use mo_timeline::*;
use mo_xml::XmlLimits;
use std::collections::BTreeSet;

fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn fixture(mode: &str) -> String {
    format!(
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst>
<p:animMotion origin="layout" path="M 0 0 L 0.15 0"><p:cBhvr additive="repl"><p:cTn id="2" dur="1000" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="10"/></p:tgtEl></p:cBhvr></p:animMotion>
<p:animMotion origin="layout" path="M 0.02 0 L 0.1 0"><p:cBhvr{mode}><p:cTn id="3" dur="1000" fill="remove"><p:stCondLst><p:cond delay="500"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="10"/></p:tgtEl></p:cBhvr></p:animMotion>
</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#
    )
}
fn read(xml: &str) -> Result<NativeTimeline, PptxError> {
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
fn native_default_layout_path_replaces_active_or_held_offsets_and_removes_cleanly() {
    for mode in ["", " additive=\"base\"", " additive=\"repl\""] {
        let timeline = read(&fixture(mode)).unwrap().timeline;
        let plan = TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).unwrap();
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("motion-defaults").unwrap(),
            revision: Digest::from_sha256([33; 32]),
            generation: PlaybackGeneration::new(1),
        };
        let mut retained = TimelineSampler::new(plan.clone());
        // At 500 the second complete offset replaces the first; at 1500 its
        // removal reveals the first path's held endpoint. Scrub backwards too.
        for (ms, numerator, denominator) in [
            (250, "3", "80"),
            (500, "1", "50"),
            (1000, "3", "50"),
            (1500, "3", "20"),
            (2500, "3", "20"),
            (500, "1", "50"),
            (0, "0", "1"),
        ] {
            let frame = plan.evaluate(&binding, time(ms), None, &|| false).unwrap();
            let motion = &frame.state.motion[&ObjectId::new("sp.10").unwrap()];
            assert_eq!(motion.x.numerator, numerator, "{mode}/{ms}");
            assert_eq!(motion.x.denominator, denominator, "{mode}/{ms}");
            assert_eq!(motion.y.numerator, "0");
            assert_eq!(
                retained
                    .evaluate(&binding, time(ms), None, &|| false)
                    .unwrap(),
                frame
            );
        }
    }
}
#[test]
fn native_motion_default_admission_does_not_accept_other_composition_or_transform_rules() {
    for mode in ["sum", "mult", "none", "unknown"] {
        let error = read(&fixture(&format!(" additive=\"{mode}\""))).unwrap_err();
        assert!(matches!(error, PptxError::Unsupported(_)));
        assert!(error.to_string().contains("cBhvr additive"));
    }
    for attributes in [" accumulate=\"always\"", " xfrmType=\"img\""] {
        let error = read(&fixture(attributes)).unwrap_err();
        assert!(matches!(error, PptxError::Unsupported(_)));
        assert!(error.to_string().contains("cBhvr"));
    }
    // Scale has a different native default domain, still not admitted here.
    let numeric = fixture("")
        .replace("p:animMotion", "p:animScale")
        .replace("origin=\"layout\" path=\"M 0 0 L 0.15 0\"", "")
        .replace("origin=\"layout\" path=\"M 0.02 0 L 0.1 0\"", "");
    let numeric = numeric.replace(
        "</p:animScale>",
        "<p:by x=\"200000\" y=\"200000\"/></p:animScale>",
    );
    let error = read(&numeric).unwrap_err();
    assert!(matches!(error, PptxError::Unsupported(_)));
    assert!(error.to_string().contains("cBhvr additive"));
}
