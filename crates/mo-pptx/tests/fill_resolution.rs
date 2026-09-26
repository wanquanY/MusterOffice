mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{fill::resolve::*, *},
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
fn rewrite(bytes: &[u8], part: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(bytes);
    let part = PartName::new(part).unwrap();
    let xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, f(xml).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn fixture(layers: [(&str, Option<u32>); 3], theme: &str, hierarchy: bool) -> Vec<u8> {
    let (d, defaults) = support::input();
    let mut bytes = export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    for ((part, name), (fill, reference)) in [
        (SLIDE, "title:1"),
        (LAYOUT, "rule:layout"),
        (MASTER, "footer:master"),
    ]
    .into_iter()
    .zip(layers)
    {
        bytes = rewrite(&bytes, part, |mut xml| {
            let id = xml.find(&format!("name=\"{name}\"")).unwrap();
            let begin = xml[..id].rfind("<p:sp>").unwrap();
            let end = id + xml[id..].find("</p:sp>").unwrap() + 7;
            let mut shape = xml[begin..end].to_owned();
            let a = shape.find("<p:spPr>").unwrap();
            let b = a + shape[a..].find("</p:spPr>").unwrap() + 9;
            shape.replace_range(a..b, &format!("<p:spPr>{fill}</p:spPr>"));
            if let Some(i) = reference {
                shape = shape.replace("</p:spPr>", &format!("</p:spPr><p:style><a:lnRef idx=\"0\"/><a:fillRef idx=\"{i}\"><a:srgbClr val=\"1256EF\"/></a:fillRef><a:effectRef idx=\"0\"/><a:fontRef idx=\"minor\"/></p:style>"));
            }
            if hierarchy {
                shape = shape.replace(
                    "<p:nvPr/>",
                    "<p:nvPr><p:ph type=\"body\" idx=\"7\"/></p:nvPr>",
                );
            }
            xml.replace_range(begin..end, &shape);
            xml
        });
    }
    rewrite(&bytes, "/ppt/theme/theme2.xml", |mut xml| {
        let a = xml.find("<a:fillStyleLst>").unwrap() + "<a:fillStyleLst>".len();
        let b = a + xml[a..].find("</a:fillStyleLst>").unwrap();
        xml.replace_range(a..b, &format!("{theme}<a:noFill/><a:noFill/>"));
        xml
    })
}
fn inspect(bytes: &[u8]) -> SourceIndex {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false).unwrap()
}
fn id(index: &SourceIndex, part: &str, name: &str) -> u32 {
    index.surfaces[part]
        .objects
        .iter()
        .find(|o| o.name == name)
        .unwrap()
        .native_id
}
fn request(index: &SourceIndex) -> SourceFillQuery {
    SourceFillQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        targets: vec![FillTarget::Object {
            native_id: id(index, SLIDE, "title:1"),
        }],
        profile: FillProfile::Drawingml2024DraftV1,
    }
}
fn outcome(index: &SourceIndex, targets: Vec<FillTarget>) -> Value {
    let mut r = request(index);
    r.targets = targets;
    let styles = query(index, &r, FillResolveLimits::default(), &|| false).unwrap();
    serde_json::to_value(&styles.targets[0].outcome).unwrap()
}
fn resolved(index: &SourceIndex) -> Value {
    let o = outcome(index, request(index).targets);
    assert_eq!(o["status"], "resolved", "{o}");
    o["fill"].clone()
}
fn background(bytes: &[u8], part: &str, declaration: &str) -> Vec<u8> {
    rewrite(bytes, part, |mut xml| {
        if let Some(a) = xml.find("<p:bg>") {
            let b = a + xml[a..].find("</p:bg>").unwrap() + 7;
            xml.replace_range(a..b, "");
        }
        xml.replacen("<p:spTree>", &format!("{declaration}<p:spTree>"), 1)
    })
}

#[test]
fn defaults_preserve_source_absence_and_native_lexicals() {
    for (xml, kind) in [
        ("", "none"),
        ("<a:solidFill/>", "solid"),
        ("<a:gradFill/>", "gradient"),
        ("<a:pattFill/>", "pattern"),
    ] {
        let index = inspect(&fixture(
            [(xml, None), ("", None), ("", None)],
            "<a:noFill/>",
            false,
        ));
        let before = serde_json::to_value(&index).unwrap();
        let f = resolved(&index);
        assert_eq!(f["kind"], kind);
        match kind {
            "solid" => assert_eq!(f["color"]["color"]["value"]["slot"], "bg1"),
            "gradient" => {
                assert_eq!(
                    f["gradient"]["stops"]["value"][1]["position"]["value"],
                    "100000"
                );
                assert_eq!(f["gradient"]["rotateWithShape"]["value"], true);
                assert_eq!(f["gradient"]["shade"]["scaled"]["value"], false);
            }
            "pattern" => assert_eq!(f["pattern"]["preset"]["value"], "pct5"),
            _ => (),
        }
        assert_eq!(serde_json::to_value(index).unwrap(), before);
    }
}
#[test]
fn direct_theme_layout_master_properties_keep_exact_provenance() {
    let index = inspect(&fixture(
        [
            (
                "<a:gradFill rotWithShape=\"0\"><a:lin/></a:gradFill>",
                Some(1),
            ),
            (
                "<a:gradFill flip=\"x\"><a:lin scaled=\"1\"/></a:gradFill>",
                None,
            ),
            (
                "<a:gradFill><a:tileRect l=\"-12.123456789%\"/></a:gradFill>",
                None,
            ),
        ],
        "<a:gradFill><a:gsLst><a:gs pos=\"50.00%\"><a:schemeClr val=\"phClr\"><a:alphaMod val=\"12.123456789%\"/></a:schemeClr></a:gs><a:gs pos=\"+0050000\"><a:srgbClr val=\"ABCDEF\"/></a:gs></a:gsLst><a:lin ang=\"60000\"/></a:gradFill>",
        true,
    ));
    let f = resolved(&index);
    let g = &f["gradient"];
    assert_eq!(g["rotateWithShape"]["value"], false);
    assert_eq!(g["flip"]["declaredBy"]["owner"]["part"], LAYOUT);
    assert_eq!(g["shade"]["angle"]["declaredBy"]["kind"], "theme");
    assert_eq!(g["shade"]["scaled"]["declaredBy"]["owner"]["part"], LAYOUT);
    assert_eq!(g["tileRect"]["left"]["value"], "-12.123456789%");
    assert_eq!(g["tileRect"]["left"]["declaredBy"]["owner"]["part"], MASTER);
    assert_eq!(
        g["tileRect"]["right"]["declaredBy"]["kind"],
        "schemaDefault"
    );
    let s = &g["stops"]["value"];
    assert_eq!(s[0]["position"]["value"], "50.00%");
    assert_eq!(s[1]["position"]["value"], "+0050000");
    assert_eq!(
        s[0]["color"]["color"]["transforms"][0]["value"],
        "12.123456789%"
    );
    assert_eq!(s[0]["color"]["contextOwner"]["part"], SLIDE);
}
#[test]
fn property_choices_gate_merging_and_present_rectangles_receive_schema_defaults() {
    let index = inspect(&fixture(
        [
            (
                "<a:gradFill><a:path><a:fillToRect l=\"0\"/></a:path></a:gradFill>",
                Some(1),
            ),
            (
                "<a:gradFill><a:path path=\"circle\"><a:fillToRect t=\"25%\"/></a:path></a:gradFill>",
                None,
            ),
            ("", None),
        ],
        "<a:gradFill><a:lin ang=\"120000\"/></a:gradFill>",
        true,
    ));
    let f = resolved(&index);
    let s = &f["gradient"]["shade"];
    assert_eq!(s["kind"], "path");
    assert_eq!(s["path"]["value"], "circle");
    assert_eq!(s["fillToRect"]["top"]["value"], "25%");
    let index = inspect(&fixture(
        [
            ("<a:gradFill><a:path/></a:gradFill>", Some(1)),
            ("", None),
            ("", None),
        ],
        "<a:pattFill/>",
        false,
    ));
    let f = resolved(&index);
    assert_eq!(
        f["gradient"]["shade"]["fillToRect"]["top"]["value"],
        "50000"
    );
}
#[test]
fn no_fill_and_complete_colors_do_not_consult_unused_references() {
    for xml in [
        "<a:noFill/>",
        "<a:solidFill><a:srgbClr val=\"123456\"/></a:solidFill>",
    ] {
        let index = inspect(&fixture(
            [(xml, Some(u32::MAX)), ("", None), ("", None)],
            "<a:noFill/>",
            false,
        ));
        resolved(&index);
    }
    for i in [0, 1000] {
        let index = inspect(&fixture(
            [("", Some(i)), ("<a:solidFill/>", None), ("", None)],
            "<a:solidFill/>",
            true,
        ));
        assert_eq!(resolved(&index)["kind"], "none");
    }
}
#[test]
fn native_stops_are_atomic_and_not_sorted_or_deduplicated() {
    let stops = "<a:gsLst><a:gs pos=\"100000\"><a:srgbClr val=\"010203\"/></a:gs><a:gs pos=\"0\"><a:srgbClr val=\"040506\"/></a:gs><a:gs pos=\"0\"><a:srgbClr val=\"070809\"/></a:gs></a:gsLst>";
    let fill = format!("<a:gradFill>{stops}</a:gradFill>");
    let index = inspect(&fixture(
        [(&fill, Some(1)), ("", None), ("", None)],
        "<a:gradFill/>",
        false,
    ));
    let f = resolved(&index);
    let s = f["gradient"]["stops"]["value"].as_array().unwrap();
    assert_eq!(
        s.iter()
            .map(|s| s["position"]["value"].clone())
            .collect::<Vec<_>>(),
        vec![json!("100000"), json!("0"), json!("0")]
    );
}
#[test]
fn image_fields_keep_relationship_parts_and_do_not_merge_stretch_into_tile() {
    let index = inspect(&fixture(
        [
            (
                "<a:blipFill><a:tile tx=\"-0.000000001cm\"/></a:blipFill>",
                Some(1),
            ),
            (
                "<a:blipFill><a:blip r:link=\"from-layout\"/><a:tile sy=\"-20%\"/></a:blipFill>",
                None,
            ),
            ("", None),
        ],
        "<a:blipFill dpi=\"72\"><a:blip xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" r:embed=\"from-theme\"/><a:stretch><a:fillRect b=\"10%\"/></a:stretch></a:blipFill>",
        true,
    ));
    let f = resolved(&index);
    let i = &f["image"];
    assert_eq!(i["embed"]["declaredBy"]["part"], "/ppt/theme/theme2.xml");
    assert_eq!(i["link"]["declaredBy"]["owner"]["part"], LAYOUT);
    assert_eq!(i["mode"]["kind"], "tile");
    assert_eq!(i["mode"]["tile"]["translateX"]["value"], "-0.000000001cm");
    assert_eq!(i["mode"]["tile"]["scaleY"]["value"], "-20%");
}
#[test]
fn background_uses_nearest_whole_declaration_and_does_not_fall_through_no_fill() {
    let mut b = fixture([("", None); 3], "<a:noFill/>", false);
    for part in [SLIDE, LAYOUT, MASTER] {
        b = background(&b, part, "");
    }
    let target = vec![FillTarget::Background {}];
    assert_eq!(
        outcome(&inspect(&b), target.clone())["fill"]["kind"],
        "none"
    );
    b = background(
        &b,
        MASTER,
        "<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"AABBCC\"/></a:solidFill></p:bgPr></p:bg>",
    );
    assert_eq!(
        outcome(&inspect(&b), target.clone())["fill"]["declaredBy"]["owner"]["part"],
        MASTER
    );
    b = background(&b, LAYOUT, "<p:bg><p:bgPr><a:noFill/></p:bgPr></p:bg>");
    assert_eq!(outcome(&inspect(&b), target)["fill"]["kind"], "none");
}
#[test]
fn shape_background_flag_redirects_before_local_shape_fill() {
    let b = fixture([("<a:noFill/>", None); 3], "<a:noFill/>", false);
    let b = background(
        &b,
        SLIDE,
        "<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"123456\"/></a:solidFill></p:bgPr></p:bg>",
    );
    for (flag, expected) in [("true", "solid"), ("0", "none")] {
        let index = inspect(&rewrite(&b, SLIDE, |x| {
            x.replacen("<p:sp>", &format!("<p:sp useBgFill=\"{flag}\">"), 1)
        }));
        let o = outcome(&index, request(&index).targets);
        assert_eq!(o["fill"]["kind"], expected);
        assert_eq!(
            o["redirects"].as_array().unwrap().len(),
            usize::from(flag == "true")
        );
        assert!(
            index.surfaces[SLIDE].objects[0]
                .use_background_fill
                .is_some()
        );
    }
}
#[test]
fn groups_follow_physical_ancestors_and_root_is_a_valid_fill_owner() {
    let b = fixture(
        [("<a:grpFill/>", None), ("", None), ("", None)],
        "<a:noFill/>",
        false,
    );
    let b = rewrite(&b, SLIDE, |mut x| {
        let a = x.find("<p:sp>").unwrap();
        let z = a + x[a..].find("</p:sp>").unwrap() + 7;
        let group = format!(
            "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"900\" name=\"group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:solidFill><a:srgbClr val=\"AABBCC\"/></a:solidFill></p:grpSpPr>{}</p:grpSp>",
            &x[a..z]
        );
        x.replace_range(a..z, &group);
        x
    });
    let index = inspect(&b);
    let o = outcome(&index, request(&index).targets);
    assert_eq!(
        o["fill"]["color"]["color"]["value"]["rgb"],
        json!([170, 187, 204])
    );
    assert_eq!(o["redirects"][0]["target"]["target"]["nativeId"], 900);
    let b = rewrite(&b, SLIDE, |x| {
        x.replace(
            "<a:solidFill><a:srgbClr val=\"AABBCC\"/></a:solidFill>",
            "<a:grpFill/>",
        )
    });
    let i = inspect(&b);
    let o = outcome(&i, request(&i).targets);
    assert_eq!(o["fill"]["kind"], "none");
    assert_eq!(o["redirects"].as_array().unwrap().len(), 2);
    let mut i = inspect(&b);
    let objects = &mut i.surfaces.get_mut(SLIDE).unwrap().objects;
    objects
        .iter_mut()
        .find(|o| o.native_id == 900)
        .unwrap()
        .parent_group = Some(900);
    assert!(matches!(
        query(&i, &request(&i), FillResolveLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}
#[test]
fn picture_placeholder_can_inherit_through_shape_placeholder_without_using_its_frame_fill() {
    let b = fixture(
        [("", None), ("<a:solidFill/>", None), ("", None)],
        "<a:noFill/>",
        true,
    );
    let b = rewrite(&b, SLIDE, |x| {
        let a = x.find("<p:pic>").unwrap();
        let z = a + x[a..].find("</p:pic>").unwrap() + 8;
        let pic = x[a..z].replace(
            "<p:nvPr/>",
            "<p:nvPr><p:ph type=\"body\" idx=\"7\"/></p:nvPr>",
        );
        let mut x = x;
        x.replace_range(a..z, &pic);
        x
    });
    let index = inspect(&b);
    let id = index.surfaces[SLIDE]
        .objects
        .iter()
        .find(|o| o.kind == SourceObjectKind::Picture)
        .unwrap()
        .native_id;
    let o = outcome(&index, vec![FillTarget::Picture { native_id: id }]);
    assert_eq!(o["status"], "resolved", "{o}");
    assert_eq!(o["fill"]["kind"], "image");
    assert_eq!(
        outcome(
            &index,
            vec![FillTarget::Picture {
                native_id: crate::id(&index, SLIDE, "title:1")
            }]
        )["reason"]["kind"],
        "unsupportedTarget"
    );
}
#[test]
fn typed_unresolved_does_not_synthesize_success_for_missing_or_retained_content() {
    for (xml, reference, kind) in [
        ("<a:blipFill/>", None, "missingImage"),
        ("<a:solidFill future=\"x\"/>", None, "retainedContent"),
        ("", Some(4), "styleIndexOutOfRange"),
    ] {
        let i = inspect(&fixture(
            [(xml, reference), ("", None), ("", None)],
            "<a:noFill/>",
            false,
        ));
        assert_eq!(outcome(&i, request(&i).targets)["reason"]["kind"], kind);
    }
    let b = background(
        &fixture([("", None); 3], "<a:noFill/>", false),
        SLIDE,
        "<p:bg><p:bgPr><a:noFill/><a:effectLst/></p:bgPr></p:bg>",
    );
    assert_eq!(
        outcome(&inspect(&b), vec![FillTarget::Background {}])["fill"]["kind"],
        "none"
    );
}

#[test]
fn only_an_explicit_known_empty_background_list_is_safe_without_effect_evaluation() {
    for (effects, reason) in [
        ("<a:effectLst/>", None),
        (
            "<a:effectLst><a:blur/></a:effectLst>",
            Some("effectEvaluationRequired"),
        ),
        ("<a:effectDag/>", Some("effectEvaluationRequired")),
        ("<a:effectLst future=\"x\"/>", Some("retainedContent")),
    ] {
        let b = background(
            &fixture([("", None); 3], "<a:noFill/>", false),
            SLIDE,
            &format!("<p:bg><p:bgPr><a:noFill/>{effects}</p:bgPr></p:bg>"),
        );
        let i = inspect(&b);
        let result = outcome(&i, vec![FillTarget::Background {}]);
        if let Some(reason) = reason {
            assert_eq!(result["reason"]["kind"], reason);
        } else {
            assert_eq!(result["status"], "resolved");
        }
    }
    let i = inspect(&fixture(
        [
            (
                "<a:blipFill><a:blip r:embed=\"owned\"><a:grayscl/></a:blip></a:blipFill>",
                None,
            ),
            ("", None),
            ("", None),
        ],
        "<a:noFill/>",
        false,
    ));
    assert_eq!(
        outcome(&i, request(&i).targets)["reason"]["kind"],
        "effectEvaluationRequired"
    );
}
#[test]
fn shared_budgets_cancellation_and_source_binding_fail_before_partial_results_escape() {
    let i = inspect(&fixture([("<a:gradFill/>", None); 3], "<a:noFill/>", false));
    let r = request(&i);
    for l in [
        FillResolveLimits {
            max_queries: 0,
            ..Default::default()
        },
        FillResolveLimits {
            max_steps: 0,
            ..Default::default()
        },
        FillResolveLimits {
            max_values: 0,
            ..Default::default()
        },
        FillResolveLimits {
            max_lexical_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(&i, &r, l, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    assert!(matches!(
        query(&i, &r, FillResolveLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    let mut bad = r.clone();
    bad.expected_source_sha256 = mo_common::Digest::from_sha256([7; 32]);
    assert!(matches!(
        query(&i, &bad, FillResolveLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
    let mut duplicate = r.clone();
    duplicate.targets.extend(r.targets.clone());
    let result = query(&i, &duplicate, FillResolveLimits::default(), &|| false).unwrap();
    assert_eq!(
        serde_json::to_value(&result.targets[0]).unwrap(),
        serde_json::to_value(&result.targets[1]).unwrap()
    );
}

#[test]
fn all_queries_share_value_budget_and_redirects_have_their_own_bound() {
    let i = inspect(&fixture([("<a:gradFill/>", None); 3], "<a:noFill/>", false));
    let mut r = request(&i);
    let limits = FillResolveLimits {
        max_values: 64,
        ..Default::default()
    };
    query(&i, &r, limits, &|| false).unwrap();
    r.targets.extend(r.targets.clone());
    assert!(matches!(
        query(&i, &r, limits, &|| false),
        Err(PptxError::Limit("fill resolution values"))
    ));
    let i = inspect(&fixture([("<a:grpFill/>", None); 3], "<a:noFill/>", false));
    let limits = FillResolveLimits {
        max_group_hops: 0,
        ..Default::default()
    };
    assert!(matches!(
        query(&i, &request(&i), limits, &|| false),
        Err(PptxError::Limit("fill redirect hops"))
    ));
}

#[test]
fn source_binding_errors_are_distinct_from_unresolved_native_semantics() {
    let mut i = inspect(&fixture(
        [("", Some(1)), ("", None), ("", None)],
        "<a:noFill/>",
        true,
    ));
    i.surfaces.get_mut(SLIDE).unwrap().theme_selection.format = None;
    assert_eq!(
        outcome(&i, request(&i).targets)["reason"]["kind"],
        "missingFormatScheme"
    );
    let mut i = inspect(&fixture([("", None); 3], "<a:noFill/>", true));
    let object_id = id(&i, SLIDE, "title:1");
    let obj = i
        .surfaces
        .get_mut(SLIDE)
        .unwrap()
        .objects
        .iter_mut()
        .find(|o| o.native_id == object_id)
        .unwrap();
    obj.resolution.placeholder_match = SourcePlaceholderMatch::Unmatched;
    assert_eq!(
        outcome(&i, request(&i).targets)["reason"]["kind"],
        "placeholder"
    );
    let mut i = inspect(&fixture([("", None); 3], "<a:noFill/>", true));
    i.surfaces.get_mut(LAYOUT).unwrap().kind = SurfaceKind::Slide;
    assert!(matches!(
        query(&i, &request(&i), FillResolveLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}

#[test]
fn invalid_native_background_flag_is_not_coerced_to_false() {
    let b = fixture([("", None); 3], "<a:noFill/>", false);
    let b = rewrite(&b, SLIDE, |x| {
        x.replacen("<p:sp>", "<p:sp useBgFill=\"yes\">", 1)
    });
    assert!(inspect_source(&package(&b), SourceLimits::default(), &|| false).is_err());
}
