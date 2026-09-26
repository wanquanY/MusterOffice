mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{
        text::{body::*, *},
        theme::SourceThemeDefaultKind,
        *,
    },
    *,
};
const SLIDE: &str = "/ppt/slides/slide1.xml";
const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";
const THEME: &str = "/ppt/theme/theme2.xml";
fn package(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap()
}
fn base() -> Vec<u8> {
    let (d, v) = support::input();
    export(
        &d,
        &v,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn rewrite(b: &[u8], part: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(b);
    let part = PartName::new(part).unwrap();
    let s = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, f(s).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn body(b: &[u8], part: &str, props: &str) -> Vec<u8> {
    rewrite(b, part, |mut s| {
        let a = s.find("<p:txBody>").unwrap();
        let z = a + s[a..].find("</p:txBody>").unwrap() + 11;
        s.replace_range(a..z,&format!("<p:txBody>{props}<a:lstStyle/><a:p><a:r><a:t>Native text</a:t></a:r></a:p></p:txBody>"));
        s
    })
}
fn themed(b: &[u8], defs: &str) -> Vec<u8> {
    rewrite(b, THEME, |s| {
        s.replace(
            "</a:theme>",
            &format!("<a:objectDefaults>{defs}</a:objectDefaults></a:theme>"),
        )
    })
}
fn definition(kind: &str, body: &str, list: &str) -> String {
    format!("<a:{kind}><a:spPr/>{body}<a:lstStyle>{list}</a:lstStyle></a:{kind}>")
}
fn inspect(b: &[u8]) -> SourceIndex {
    inspect_source(&package(b), SourceLimits::default(), &|| false).unwrap()
}
fn request(i: &SourceIndex) -> SourceTextBodyQuery {
    SourceTextBodyQuery {
        expected_source_sha256: i.source_sha256.clone(),
        surface: SLIDE.into(),
        objects: vec![i.surfaces[SLIDE].objects[0].native_id],
        profile: TextBodyProfile::DrawingmlDraftV1,
    }
}
fn resolve(i: &SourceIndex) -> EffectiveTextBody {
    let r = query(i, &request(i), TextBodyLimits::default(), &|| false).unwrap();
    let TextBodyOutcome::Resolved { body } = &r.objects[0].outcome else {
        panic!("{:?}", r.objects)
    };
    *body.clone()
}
#[test]
fn body_defaults_are_explicit_and_do_not_mutate_author_declarations() {
    let i = inspect(&body(&base(), SLIDE, "<a:bodyPr/>"));
    let before = i.clone();
    let e = resolve(&i);
    assert_eq!(e.origins.len(), 19);
    assert!(
        e.origins
            .values()
            .all(|o| matches!(o, TextBodyOrigin::ProfileDefault {}))
    );
    assert_eq!(e.attributes.left_inset.as_ref().unwrap().lexical(), "91440");
    assert_eq!(e.attributes.top_inset.as_ref().unwrap().lexical(), "45720");
    assert_eq!(e.attributes.columns, Some(1));
    assert_eq!(e.attributes.wrap, Some(NativeTextWrap::Square));
    assert_eq!(e.attributes.paragraph_spacing, Some(false));
    assert_eq!(e.attributes.rotation, Some(0));
    assert!(matches!(
        e.autofit,
        EffectiveTextAutofit::None {
            declared_by: TextBodyOrigin::ProfileDefault {}
        }
    ));
    assert_eq!(i, before);
}
#[test]
fn themes_supply_per_property_values_in_text_line_shape_order() {
    let b = body(&base(), SLIDE, r#"<a:bodyPr lIns="0" rtlCol="false"/>"#);
    let defs = definition("spDef", r#"<a:bodyPr tIns="5" bIns="6" rIns="7"/>"#, "")
        + &definition("lnDef", r#"<a:bodyPr tIns="3" bIns="4"/>"#, "")
        + &definition(
            "txDef",
            r#"<a:bodyPr tIns="0.000000001pt" lIns="2" rtlCol="1"><a:normAutofit fontScale="92.00000000001%"/></a:bodyPr>"#,
            r#"<a:lvl1pPr><a:defRPr sz="2400"/></a:lvl1pPr>"#,
        );
    let i = inspect(&themed(&b, &defs));
    let e = resolve(&i);
    assert_eq!(e.attributes.left_inset.as_ref().unwrap().lexical(), "0");
    assert_eq!(
        e.attributes.top_inset.as_ref().unwrap().lexical(),
        "0.000000001pt"
    );
    assert_eq!(e.attributes.bottom_inset.as_ref().unwrap().lexical(), "4");
    assert_eq!(e.attributes.right_inset.as_ref().unwrap().lexical(), "7");
    assert_eq!(e.attributes.right_to_left_columns, Some(false));
    for (key, kind) in [
        (TextBodyProperty::TopInset, SourceThemeDefaultKind::Text),
        (TextBodyProperty::BottomInset, SourceThemeDefaultKind::Line),
        (TextBodyProperty::RightInset, SourceThemeDefaultKind::Shape),
    ] {
        assert!(
            matches!(&e.origins[&key],TextBodyOrigin::Theme {part, default_kind, ..} if part==THEME && *default_kind==Some(kind))
        );
    }
    let EffectiveTextAutofit::Normal {
        font_scale,
        line_spacing_reduction,
        font_scale_defaulted,
        line_spacing_reduction_defaulted,
        ..
    } = e.autofit
    else {
        panic!()
    };
    assert_eq!(font_scale.lexical(), "92.00000000001%");
    assert_eq!(line_spacing_reduction.lexical(), "0");
    assert!(!font_scale_defaulted && line_spacing_reduction_defaulted);
    let defaults = i.themes[THEME].text_defaults.as_ref().unwrap();
    assert_eq!(defaults.entries.len(), 3);
    assert!(
        defaults.entries[&SourceThemeDefaultKind::Text]
            .text
            .nodes
            .values()
            .any(|n| n.element == NativeTextElement::DefRPr)
    );
}
#[test]
fn explicit_autofit_choices_replace_the_inherited_choice() {
    let def = definition(
        "txDef",
        r#"<a:bodyPr><a:normAutofit fontScale="80000" lnSpcReduction="10000"/></a:bodyPr>"#,
        "",
    );
    for (xml, kind) in [
        ("<a:noAutofit/>", 0),
        ("<a:spAutoFit/>", 1),
        ("<a:normAutofit/>", 2),
    ] {
        let i = inspect(&themed(
            &body(&base(), SLIDE, &format!("<a:bodyPr>{xml}</a:bodyPr>")),
            &def,
        ));
        match (kind, resolve(&i).autofit) {
            (
                0,
                EffectiveTextAutofit::None {
                    declared_by: TextBodyOrigin::Object { .. },
                },
            )
            | (
                1,
                EffectiveTextAutofit::Shape {
                    declared_by: TextBodyOrigin::Object { .. },
                },
            ) => (),
            (
                2,
                EffectiveTextAutofit::Normal {
                    font_scale,
                    line_spacing_reduction,
                    font_scale_defaulted: true,
                    line_spacing_reduction_defaulted: true,
                    ..
                },
            ) => {
                assert_eq!(font_scale.lexical(), "100000");
                assert_eq!(line_spacing_reduction.lexical(), "0")
            }
            other => panic!("{other:?}"),
        }
    }
}
fn hierarchy() -> Vec<u8> {
    let b = body(&base(), SLIDE, r#"<a:bodyPr lIns="11"/>"#);
    let b = rewrite(&b, SLIDE, |s| {
        let at = s.find("<p:sp>").unwrap();
        s[..at].to_owned()
            + &s[at..].replacen(
                "<p:nvPr/>",
                "<p:nvPr><p:ph type=\"body\" idx=\"0\"/></p:nvPr>",
                1,
            )
    });
    let s = String::from_utf8(
        package(&b)
            .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
            .unwrap(),
    )
    .unwrap();
    let a = s.find("<p:sp>").unwrap();
    let z = a + s[a..].find("</p:sp>").unwrap() + 7;
    let shape = s[a..z].to_owned();
    let mut b = b;
    for (part, props) in [
        (LAYOUT, r#"<a:bodyPr tIns="22"/>"#),
        (MASTER, r#"<a:bodyPr tIns="33" bIns="44" rIns="55"/>"#),
    ] {
        let mut shape = shape.clone();
        let a = shape.find("<p:txBody>").unwrap() + 10;
        let z = a + shape[a..].find("<a:lstStyle").unwrap();
        shape.replace_range(a..z, props);
        b = rewrite(&b, part, |s| {
            s.replace("</p:spTree>", &format!("{shape}</p:spTree>"))
        });
    }
    b
}
#[test]
fn layout_placeholder_then_theme_then_master_supply_remaining_attributes() {
    let b = themed(
        &hierarchy(),
        &definition("txDef", r#"<a:bodyPr rIns="66"/>"#, ""),
    );
    let i = inspect(&b);
    let e = resolve(&i);
    assert_eq!(e.attributes.left_inset.as_ref().unwrap().lexical(), "11");
    assert_eq!(e.attributes.top_inset.as_ref().unwrap().lexical(), "22");
    assert_eq!(e.attributes.bottom_inset.as_ref().unwrap().lexical(), "44");
    assert_eq!(e.attributes.right_inset.as_ref().unwrap().lexical(), "66");
    assert!(
        matches!(&e.origins[&TextBodyProperty::TopInset],TextBodyOrigin::Object{object,..} if object.part==LAYOUT)
    );
    assert!(
        matches!(&e.origins[&TextBodyProperty::BottomInset],TextBodyOrigin::Object{object,..} if object.part==MASTER)
    );
}
#[test]
fn unresolved_placeholders_unknown_body_content_and_absent_bodies_do_not_synthesize_success() {
    let mut i = inspect(&body(&base(), SLIDE, "<a:bodyPr/>"));
    let id = i.surfaces[SLIDE].objects[0].native_id;
    i.surfaces.get_mut(SLIDE).unwrap().objects[0]
        .resolution
        .placeholder_match = SourcePlaceholderMatch::Ambiguous {
        part: LAYOUT.into(),
        candidates: 2,
    };
    let q = request(&i);
    assert!(matches!(
        query(&i, &q, TextBodyLimits::default(), &|| false)
            .unwrap()
            .objects[0]
            .outcome,
        TextBodyOutcome::Unresolved {
            reason: TextBodyUnresolved::Placeholder { .. }
        }
    ));
    let i = inspect(&body(&base(), SLIDE, r#"<a:bodyPr surprise="1"/>"#));
    assert!(matches!(
        query(&i, &request(&i), TextBodyLimits::default(), &|| false)
            .unwrap()
            .objects[0]
            .outcome,
        TextBodyOutcome::Unresolved {
            reason: TextBodyUnresolved::RetainedContent { .. }
        }
    ));
    let mut i = inspect(&base());
    i.surfaces
        .get_mut(SLIDE)
        .unwrap()
        .objects
        .iter_mut()
        .find(|o| o.native_id == id)
        .unwrap()
        .text_body_ordinal = None;
    assert!(matches!(
        query(&i, &request(&i), TextBodyLimits::default(), &|| false)
            .unwrap()
            .objects[0]
            .outcome,
        TextBodyOutcome::Unresolved {
            reason: TextBodyUnresolved::NoTextBody {}
        }
    ));
}
#[test]
fn source_conflicts_limits_cancellation_and_duplicates_are_handled_atomically() {
    let i = inspect(&hierarchy());
    let mut q = request(&i);
    q.objects.push(q.objects[0]);
    let r = query(&i, &q, TextBodyLimits::default(), &|| false).unwrap();
    assert_eq!(r.objects[0].outcome, r.objects[1].outcome);
    assert!(matches!(
        query(
            &i,
            &q,
            TextBodyLimits {
                max_queries: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    assert!(matches!(
        query(
            &i,
            &q,
            TextBodyLimits {
                max_steps: 0,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    assert!(matches!(
        query(
            &i,
            &q,
            TextBodyLimits {
                max_lexical_bytes: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    assert!(query(&i, &q, TextBodyLimits::default(), &|| true).is_err());
    q.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(&i, &q, TextBodyLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}
#[test]
fn theme_siblings_cannot_inject_text_and_unknown_default_content_is_reported() {
    let b = body(&base(), SLIDE, "<a:bodyPr/>");
    let defs = definition("txDef", "<a:bodyPr/>", "")
        .replace("<a:spPr/>", r#"<a:spPr><a:bodyPr lIns="999"/></a:spPr>"#);
    let i = inspect(&themed(&b, &defs));
    assert_eq!(
        resolve(&i).attributes.left_inset.unwrap().lexical(),
        "91440"
    );
    let i = inspect(&themed(&b, &(defs + "<a:extLst/>")));
    assert!(matches!(
        query(&i, &request(&i), TextBodyLimits::default(), &|| false)
            .unwrap()
            .objects[0]
            .outcome,
        TextBodyOutcome::Unresolved {
            reason: TextBodyUnresolved::RetainedContent { .. }
        }
    ));
}
#[test]
fn theme_text_budgets_share_the_surface_counter_and_duplicate_defaults_fail() {
    let b = themed(&base(), &definition("txDef", "<a:bodyPr/>", ""));
    let p = package(&b);
    assert!(
        inspect_source(
            &p,
            SourceLimits {
                max_text_style_elements: 0,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    let b = themed(&base(), &definition("txDef", "<a:bodyPr/>", ""));
    let b = rewrite(&b, THEME, |s| {
        s.replace("</a:theme>", "<a:objectDefaults/></a:theme>")
    });
    assert!(inspect_source(&package(&b), SourceLimits::default(), &|| false).is_err());
}
