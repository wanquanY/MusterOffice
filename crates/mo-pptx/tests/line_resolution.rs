mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{
        line::{resolve::*, *},
        *,
    },
    *,
};
use serde_json::{Value, json};

const SLIDE: &str = "/ppt/slides/slide1.xml";
const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn fixture(layers: [(&str, Option<u32>); 3], theme: &str, hierarchy: bool) -> SourceIndex {
    let (d, defaults) = support::input();
    let bytes = export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&bytes);
    let mut plan = RewritePlan::new();
    for ((part, name), (line, reference)) in [
        (SLIDE, "title:1"),
        (LAYOUT, "rule:layout"),
        (MASTER, "footer:master"),
    ]
    .into_iter()
    .zip(layers)
    {
        let part = PartName::new(part).unwrap();
        let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
        let id = xml.find(&format!("name=\"{name}\"")).unwrap();
        let begin = xml[..id].rfind("<p:sp>").unwrap();
        let end = id + xml[id..].find("</p:sp>").unwrap() + 7;
        let mut shape = xml[begin..end].to_owned();
        let a = shape.find("<p:spPr>").unwrap();
        let b = a + shape[a..].find("</p:spPr>").unwrap() + 9;
        shape.replace_range(a..b, &format!("<p:spPr>{line}</p:spPr>"));
        if let Some(index) = reference {
            shape = shape.replace("</p:spPr>", &format!("</p:spPr><p:style><a:lnRef idx=\"{index}\"><a:srgbClr val=\"1256EF\"/></a:lnRef></p:style>"));
        }
        if hierarchy {
            shape = shape.replace(
                "<p:nvPr/>",
                "<p:nvPr><p:ph type=\"body\" idx=\"7\"/></p:nvPr>",
            );
        }
        xml.replace_range(begin..end, &shape);
        plan.replace_part(part, xml.into_bytes()).unwrap();
    }
    let part = PartName::new("/ppt/theme/theme2.xml").unwrap();
    let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let a = xml.find("<a:lnStyleLst>").unwrap() + "<a:lnStyleLst>".len();
    let b = a + xml[a..].find("</a:lnStyleLst>").unwrap();
    xml.replace_range(a..b, &format!("{theme}<a:ln/><a:ln/>"));
    plan.replace_part(part, xml.into_bytes()).unwrap();
    let bytes = plan.to_bytes(&p, &|| false).unwrap();
    inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap()
}
fn request(index: &SourceIndex) -> SourceLineQuery {
    SourceLineQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        objects: vec![
            index.surfaces[SLIDE]
                .objects
                .iter()
                .find(|o| o.name == "title:1")
                .unwrap()
                .native_id,
        ],
        profile: LineProfile::Drawingml2024DraftV1,
    }
}
fn resolved(index: &SourceIndex) -> Value {
    let styles = query(
        index,
        &request(index),
        LineResolveLimits::default(),
        &|| false,
    )
    .unwrap();
    let LineOutcome::Resolved { line } = &styles.objects[0].outcome else {
        panic!("{:?}", styles.objects[0]);
    };
    serde_json::to_value(line).unwrap()
}
#[test]
fn profile_defaults_are_derived_and_do_not_overwrite_absent_declarations() {
    let index = fixture([("", None); 3], "<a:ln/>", false);
    let before = serde_json::to_value(&index).unwrap();
    let line = resolved(&index);
    assert_eq!(line["width"]["value"], "9525");
    assert_eq!(line["cap"]["value"], "flat");
    assert_eq!(line["join"]["kind"], "round");
    assert_eq!(line["fill"]["kind"], "none");
    assert_eq!(line["tail"]["width"]["value"], "med");
    assert_eq!(line["width"]["declaredBy"]["kind"], "profileDefault");
    assert_eq!(serde_json::to_value(index).unwrap(), before);
}
#[test]
fn direct_style_and_parent_layers_fill_individual_gaps_with_provenance() {
    let index = fixture(
        [
            (
                r#"<a:ln w="0"><a:solidFill/><a:miter/><a:headEnd type="triangle"/></a:ln>"#,
                Some(1),
            ),
            (
                r#"<a:ln w="22222" cap="flat" cmpd="dbl"><a:miter lim="600000"/><a:headEnd w="lg"/></a:ln>"#,
                None,
            ),
            (
                r#"<a:ln algn="in"><a:tailEnd type="arrow" len="lg"/></a:ln>"#,
                None,
            ),
        ],
        r#"<a:ln w="33333" cap="rnd"><a:solidFill><a:schemeClr val="phClr"><a:alphaMod val="12.123456789%"/></a:schemeClr></a:solidFill><a:miter lim="400000"/></a:ln>"#,
        true,
    );
    let line = resolved(&index);
    assert_eq!(line["width"]["value"], "0");
    assert_eq!(line["cap"]["value"], "rnd");
    assert_eq!(line["cap"]["declaredBy"]["kind"], "theme");
    assert_eq!(line["compound"]["declaredBy"]["object"]["part"], LAYOUT);
    assert_eq!(line["alignment"]["declaredBy"]["object"]["part"], MASTER);
    assert_eq!(line["join"]["limit"]["value"], "400000");
    assert_eq!(line["join"]["declaredBy"]["object"]["part"], SLIDE);
    assert_eq!(line["head"]["kind"]["value"], "triangle");
    assert_eq!(line["head"]["width"]["value"], "lg");
    assert_eq!(line["head"]["length"]["value"], "med");
    assert_eq!(
        line["fill"]["color"]["color"]["transforms"][0]["value"],
        "12.123456789%"
    );
    assert_eq!(
        line["fill"]["color"]["placeholder"]["value"]["rgb"],
        json!([18, 86, 239])
    );
}
#[test]
fn choices_do_not_merge_across_kinds_and_empty_custom_dash_is_atomic() {
    let index = fixture(
        [
            ("<a:ln><a:noFill/><a:custDash/><a:round/></a:ln>", Some(1)),
            ("", None),
            ("", None),
        ],
        r#"<a:ln w="50000"><a:gradFill/><a:custDash><a:ds d="10" sp="20"/></a:custDash><a:miter lim="0"/></a:ln>"#,
        false,
    );
    let line = resolved(&index);
    assert_eq!(line["fill"]["kind"], "none");
    assert_eq!(line["dash"]["stops"], json!([]));
    assert_eq!(line["join"]["kind"], "round");
    assert_eq!(line["width"]["value"], "50000");
}
#[test]
fn empty_miter_merges_same_kind_and_falls_back_to_documented_limit() {
    for (theme, expected) in [
        ("<a:ln/>", "800000"),
        ("<a:ln><a:round/></a:ln>", "800000"),
        ("<a:ln><a:miter lim=\"0\"/></a:ln>", "0"),
    ] {
        let index = fixture(
            [("<a:ln><a:miter/></a:ln>", Some(1)), ("", None), ("", None)],
            theme,
            false,
        );
        assert_eq!(resolved(&index)["join"]["limit"]["value"], expected);
        let authored = index.surfaces[SLIDE]
            .objects
            .iter()
            .find(|o| o.name == "title:1")
            .unwrap()
            .line
            .as_ref()
            .unwrap();
        assert!(matches!(
            authored.join,
            Some(SourceLineJoin::Miter { limit: None, .. })
        ));
    }
}
#[test]
fn zero_style_index_does_not_become_a_synthetic_no_fill() {
    let index = fixture(
        [
            ("", Some(0)),
            (
                "<a:ln w=\"12345\"><a:solidFill><a:srgbClr val=\"ABCDEF\"/></a:solidFill></a:ln>",
                None,
            ),
            ("", None),
        ],
        "<a:ln/>",
        true,
    );
    let line = resolved(&index);
    assert_eq!(line["width"]["value"], "12345");
    assert_eq!(line["fill"]["kind"], "solid");
}
#[test]
fn unresolved_native_context_is_reported_instead_of_defaulted() {
    let cases = [
        ("<a:ln><a:gradFill/></a:ln>", None, "unsupportedFill"),
        ("<a:ln owned=\"unknown\"/>", None, "retainedContent"),
        ("", Some(4), "styleIndexOutOfRange"),
    ];
    for (line, reference, expected) in cases {
        let index = fixture(
            [(line, reference), ("", None), ("", None)],
            "<a:ln/>",
            false,
        );
        let r = query(
            &index,
            &request(&index),
            LineResolveLimits::default(),
            &|| false,
        )
        .unwrap();
        let value = serde_json::to_value(r).unwrap();
        assert_eq!(value["objects"][0]["outcome"]["reason"]["kind"], expected);
    }
    let mut index = fixture([("", None); 3], "<a:ln/>", true);
    index
        .surfaces
        .get_mut(SLIDE)
        .unwrap()
        .objects
        .iter_mut()
        .find(|o| o.name == "title:1")
        .unwrap()
        .resolution
        .placeholder_match = SourcePlaceholderMatch::Ambiguous {
        part: LAYOUT.into(),
        candidates: 2,
    };
    let r = query(
        &index,
        &request(&index),
        LineResolveLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        r.objects[0].outcome,
        LineOutcome::Unresolved {
            reason: LineUnresolved::Placeholder { .. }
        }
    ));
}
#[test]
fn resolution_validates_identity_budgets_cancellation_and_repeated_queries() {
    let index = fixture(
        [
            (
                "<a:ln><a:custDash><a:ds d=\"12.123456789%\" sp=\"10\"/></a:custDash></a:ln>",
                None,
            ),
            ("", None),
            ("", None),
        ],
        "<a:ln/>",
        false,
    );
    let mut q = request(&index);
    q.objects.push(q.objects[0]);
    let r = query(&index, &q, LineResolveLimits::default(), &|| false).unwrap();
    assert_eq!(
        serde_json::to_value(&r.objects[0]).unwrap(),
        serde_json::to_value(&r.objects[1]).unwrap()
    );
    for limits in [
        LineResolveLimits {
            max_queries: 1,
            ..Default::default()
        },
        LineResolveLimits {
            max_steps: 0,
            ..Default::default()
        },
        LineResolveLimits {
            max_values: 0,
            ..Default::default()
        },
        LineResolveLimits {
            max_lexical_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(&index, &q, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    assert!(matches!(
        query(&index, &q, LineResolveLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    q.expected_source_sha256 = mo_common::Digest::from_sha256([7; 32]);
    assert!(matches!(
        query(&index, &q, LineResolveLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
    q = request(&index);
    q.objects.push(u32::MAX);
    assert!(matches!(
        query(&index, &q, LineResolveLimits::default(), &|| false),
        Err(PptxError::Value { .. })
    ));
}
