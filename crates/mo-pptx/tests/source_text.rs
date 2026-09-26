mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{text::*, *},
    *,
};
const SLIDE: &str = "/ppt/slides/slide1.xml";
fn package(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap()
}
fn base() -> Vec<u8> {
    let (d, defaults) = support::input();
    export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn rewrite(b: &[u8], part: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(b);
    let name = PartName::new(part).unwrap();
    let s = String::from_utf8(p.read_part(&name, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(name, f(s).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn body(xml: &str) -> Vec<u8> {
    rewrite(&base(), SLIDE, |mut s| {
        let a = s.find("<p:txBody>").unwrap();
        let b = a + s[a..].find("</p:txBody>").unwrap() + 11;
        s.replace_range(a..b, &format!("<p:txBody>{xml}</p:txBody>"));
        s
    })
}
fn inspect(b: &[u8]) -> SourceIndex {
    inspect_source(&package(b), SourceLimits::default(), &|| false).unwrap()
}
fn own(index: &SourceIndex) -> Vec<&SourceTextNode> {
    let s = &index.surfaces[SLIDE];
    let root = s.objects[0].text_body_ordinal.unwrap();
    s.text
        .nodes
        .iter()
        .filter_map(|(id, n)| {
            let mut p = Some(*id);
            while let Some(id) = p {
                if id == root {
                    return Some(n);
                }
                p = s.text.nodes[&id].parent;
            }
            None
        })
        .collect()
}
fn one(index: &SourceIndex, kind: NativeTextElement) -> &SourceTextNode {
    own(index).into_iter().find(|n| n.element == kind).unwrap()
}

#[test]
fn body_declarations_preserve_absence_false_zero_units_and_autofit_precision() {
    let i = inspect(&body(
        r#"<a:bodyPr rot="-21600001" lIns="-0.0000000001mm" tIns="+000" rIns="2147483647" bIns="-2147483648" numCol="16" spcCol="0" rtlCol="0" anchor="ctr" anchorCtr="false" upright="0" vert="mongolianVert" wrap="none"><a:normAutofit fontScale="92.00000000001%" lnSpcReduction="+000"/></a:bodyPr><a:p/>"#,
    ));
    let SourceTextValue::Body { attributes: a } = &one(&i, NativeTextElement::BodyPr).value else {
        panic!()
    };
    assert_eq!(a.rotation, Some(-21600001));
    assert_eq!(a.left_inset.as_ref().unwrap().lexical(), "-0.0000000001mm");
    assert_eq!(a.top_inset.as_ref().unwrap().lexical(), "+000");
    assert_eq!(a.right_to_left_columns, Some(false));
    assert_eq!(a.from_word_art, None);
    assert_eq!(a.columns, Some(16));
    assert_eq!(a.vertical, Some(NativeTextVertical::MongolianVert));
    let SourceTextValue::Autofit {
        font_scale,
        line_spacing_reduction,
    } = &one(&i, NativeTextElement::NormAutofit).value
    else {
        panic!()
    };
    assert_eq!(font_scale.as_ref().unwrap().lexical(), "92.00000000001%");
    assert_eq!(line_spacing_reduction.as_ref().unwrap().lexical(), "+000");
    let i = inspect(&body("<a:bodyPr/><a:p/>"));
    assert!(!own(&i).iter().any(|n| matches!(
        n.element,
        NativeTextElement::NoAutofit
            | NativeTextElement::NormAutofit
            | NativeTextElement::SpAutoFit
    )));
}

#[test]
fn paragraphs_runs_end_marks_and_fonts_keep_independent_explicit_properties() {
    let i = inspect(&body(
        r#"<a:bodyPr/><a:lstStyle><a:lvl9pPr lvl="8"><a:defRPr sz="2400"/></a:lvl9pPr></a:lstStyle><a:p><a:pPr marL="0" marR="51206400" lvl="8" indent="-51206400" algn="thaiDist" defTabSz="1.234567890123pt" rtl="1" eaLnBrk="0" fontAlgn="base" latinLnBrk="false" hangingPunct="true"><a:lnSpc><a:spcPct val="110.0000000001%"/></a:lnSpc><a:spcBef><a:spcPts val="0"/></a:spcBef><a:buNone/><a:tabLst><a:tab/><a:tab pos="-1" algn="dec"/></a:tabLst><a:defRPr sz="1800"/></a:pPr><a:r><a:rPr lang="zh-CN" altLang="ar-SA" sz="1234" b="0" i="1" u="wavyDbl" strike="dblStrike" kern="0" cap="small" spc="-0.000000001pt" baseline="-20.00000001%" normalizeH="false" bmk="  bookmark  "><a:latin typeface="+mn-lt" panose="00000000000000000000" pitchFamily="00" charset="-128"/><a:ea typeface=""/><a:cs typeface="Explicit complex"/><a:sym typeface="Symbol"/><a:rtl val="on"/></a:rPr><a:t> A中ع </a:t></a:r><a:br><a:rPr sz="100"/></a:br><a:endParaRPr sz="400000"/></a:p>"#,
    ));
    let SourceTextValue::Paragraph { attributes: a } = &one(&i, NativeTextElement::PPr).value
    else {
        panic!()
    };
    assert_eq!(a.left_margin, Some(0));
    assert_eq!(a.right_to_left, Some(true));
    assert_eq!(a.level, Some(8));
    assert_eq!(
        a.default_tab_size.as_ref().unwrap().lexical(),
        "1.234567890123pt"
    );
    let SourceTextValue::Character { attributes: a } = &one(&i, NativeTextElement::RPr).value
    else {
        panic!()
    };
    assert_eq!(a.size, Some(1234));
    assert_eq!(a.bold, Some(false));
    assert_eq!(a.bookmark.as_deref(), Some("  bookmark  "));
    assert!(matches!(
        a.spacing,
        Some(NativeTextPoint::UniversalMeasure { .. })
    ));
    let SourceTextValue::Font { font } = &one(&i, NativeTextElement::Ea).value else {
        panic!()
    };
    assert_eq!(font.typeface, "");
    assert_eq!(font.pitch_family, None);
    let s = &i.surfaces[SLIDE];
    assert_eq!(s.objects[0].paragraphs[0][0].text, " A中ع ");
    assert_eq!(s.objects[0].paragraphs[0][1].kind, SourceRunKind::Break);
    assert_eq!(
        own(&i)
            .iter()
            .filter(|n| n.element == NativeTextElement::DefRPr)
            .count(),
        2
    );
}

#[test]
fn text_paints_reuse_native_fill_line_effect_and_ordered_color_declarations() {
    let i = inspect(&body(
        r#"<a:bodyPr/><a:p><a:r><a:rPr><a:ln w="12700"><a:solidFill><a:srgbClr val="123456"/></a:solidFill></a:ln><a:gradFill><a:gsLst><a:gs pos="0"><a:schemeClr val="accent1"/></a:gs><a:gs pos="100000"><a:srgbClr val="FFFFFF"/></a:gs></a:gsLst></a:gradFill><a:effectLst><a:glow rad="500"><a:srgbClr val="000000"/></a:glow></a:effectLst><a:highlight><a:srgbClr val="FEDCBA"><a:alpha val="50%"/><a:tint val="20000"/></a:srgbClr></a:highlight><a:uLn w="0"><a:noFill/></a:uLn><a:uFill><a:solidFill><a:srgbClr val="AABBCC"/></a:solidFill></a:uFill></a:rPr><a:t>Editable</a:t></a:r></a:p>"#,
    ));
    assert!(matches!(
        one(&i, NativeTextElement::GradFill).value,
        SourceTextValue::Fill { .. }
    ));
    assert!(matches!(
        one(&i, NativeTextElement::ULn).value,
        SourceTextValue::Line { .. }
    ));
    let SourceTextValue::Color { color } = &one(&i, NativeTextElement::SrgbClr).value else {
        panic!()
    };
    assert_eq!(color.transforms.len(), 2);
    assert_eq!(i.surfaces[SLIDE].text.effect_nodes.len(), 1);
    assert!(own(&i).iter().all(|n| n.retained_ordinals.is_empty()));
}

#[test]
fn master_document_defaults_and_font_references_share_grammar_without_defaulting() {
    let b = rewrite(&base(), "/ppt/presentation.xml", |s| {
        let a = s.find("<p:defaultTextStyle>").unwrap();
        let b = a + s[a..].find("</p:defaultTextStyle>").unwrap() + 21;
        let mut s = s;
        s.replace_range(a..b,"<p:defaultTextStyle><a:defPPr algn=\"r\"><a:defRPr sz=\"1000\"/></a:defPPr></p:defaultTextStyle>");
        s
    });
    let master = inspect(&b)
        .surfaces
        .iter()
        .find(|(_, s)| s.kind == SurfaceKind::Master)
        .unwrap()
        .0
        .clone();
    let b = rewrite(&b, &master, |s| {
        let a = s.find("<p:txStyles>").unwrap();
        let b = a + s[a..].find("</p:txStyles>").unwrap() + 13;
        let mut s = s;
        s.replace_range(a..b,"<p:txStyles><p:titleStyle><a:lvl1pPr algn=\"ctr\"/></p:titleStyle><p:bodyStyle/><p:otherStyle/></p:txStyles>");
        s
    });
    let b = rewrite(&b, SLIDE, |s| {
        s.replacen("</p:spPr>","</p:spPr><p:style><a:lnRef idx=\"0\"/><a:fillRef idx=\"0\"/><a:effectRef idx=\"0\"/><a:fontRef idx=\"minor\"><a:schemeClr val=\"tx1\"/></a:fontRef></p:style>",1)
    });
    let i = inspect(&b);
    assert_eq!(i.text.roots.len(), 1);
    assert_eq!(i.text.roots[0].owner, None);
    assert_eq!(i.surfaces[&master].text.roots[0].owner, None);
    let s = &i.surfaces[SLIDE];
    let root = s
        .text
        .roots
        .iter()
        .find(|r| s.text.nodes[&r.source_ordinal].element == NativeTextElement::FontRef)
        .unwrap();
    assert_eq!(root.owner, Some(s.objects[0].native_id));
    assert!(matches!(
        s.text.nodes[&root.source_ordinal].value,
        SourceTextValue::FontReference {
            index: NativeFontCollectionIndex::Minor
        }
    ));
}

#[test]
fn fields_hyperlinks_and_bullets_are_data_not_executed_or_conflated_with_plain_text() {
    let i = inspect(&body(
        r#"<a:bodyPr/><a:p><a:pPr><a:buClr><a:schemeClr val="accent1"/></a:buClr><a:buSzPct val="25000"/><a:buFont typeface="Bullet"/><a:buAutoNum type="thaiNumParenBoth" startAt="32767"/></a:pPr><a:fld id="{00000000-0000-0000-0000-000000000000}" type="slidenum"><a:rPr><a:hlinkClick r:id="rIdUnresolved" action="ppaction://hlinkshowjump" tooltip=" Keep spaces "/></a:rPr><a:pPr lvl="1"/><a:t>7</a:t></a:fld></a:p>"#,
    ));
    assert!(matches!(
        one(&i, NativeTextElement::BuAutoNum).value,
        SourceTextValue::AutoNumber {
            start_at: Some(32767),
            ..
        }
    ));
    let SourceTextValue::Hyperlink { attributes: a } =
        &one(&i, NativeTextElement::HlinkClick).value
    else {
        panic!()
    };
    assert_eq!(a.relationship_id.as_deref(), Some("rIdUnresolved"));
    assert_eq!(a.tooltip.as_deref(), Some(" Keep spaces "));
    let r = &i.surfaces[SLIDE].objects[0].paragraphs[0][0];
    assert_eq!(r.kind, SourceRunKind::Field);
    assert!(!r.editable);
    assert_eq!(r.text, "7");
}

#[test]
fn malformed_native_style_choices_orders_and_ranges_fail_without_a_partial_index() {
    for xml in [
        "<a:bodyPr numCol=\"0\"/><a:p/>",
        "<a:bodyPr lIns=\"2147483648\"/><a:p/>",
        "<a:bodyPr upright=\"yes\"/><a:p/>",
        "<a:bodyPr><a:noAutofit/><a:spAutoFit/></a:bodyPr><a:p/>",
        "<a:p/><a:bodyPr/>",
        "<a:bodyPr/><a:p><a:pPr lvl=\"9\"/></a:p>",
        "<a:bodyPr/><a:p><a:pPr><a:buNone/><a:buChar char=\"x\"/></a:pPr></a:p>",
        "<a:bodyPr/><a:p><a:pPr><a:lnSpc/></a:pPr></a:p>",
        "<a:bodyPr/><a:p><a:r><a:rPr sz=\"99\"/><a:t>A</a:t></a:r></a:p>",
        "<a:bodyPr/><a:p><a:r><a:t>A</a:t><a:rPr/></a:r></a:p>",
        "<a:bodyPr/><a:p><a:r><a:rPr spc=\"400001\"/><a:t>A</a:t></a:r></a:p>",
        "<a:bodyPr/><a:p><a:r><a:rPr><a:latin typeface=\"x\"/><a:latin typeface=\"y\"/></a:rPr><a:t>A</a:t></a:r></a:p>",
    ] {
        let b = body(xml);
        assert!(
            inspect_source(&package(&b), SourceLimits::default(), &|| false).is_err(),
            "{xml}"
        );
    }
}

#[test]
fn unmodeled_text_features_retain_owner_locations_and_do_not_inject_properties() {
    let i = inspect(&body(
        r#"<a:bodyPr extra="retained"><a:prstTxWarp prst="textWave1"><a:avLst/></a:prstTxWarp><a:noAutofit/></a:bodyPr><a:p><a:pPr><a:buBlip><a:blip/></a:buBlip></a:pPr><a:r><a:rPr><a:extLst><a:ext uri="test"><a:latin typeface="must not be active"/></a:ext></a:extLst></a:rPr><a:t>A</a:t></a:r></a:p>"#,
    ));
    assert_eq!(
        one(&i, NativeTextElement::BodyPr).retained_ordinals.len(),
        2
    );
    assert_eq!(one(&i, NativeTextElement::PPr).retained_ordinals.len(), 1);
    assert_eq!(one(&i, NativeTextElement::RPr).retained_ordinals.len(), 1);
    assert!(
        !own(&i)
            .iter()
            .any(|n| n.element == NativeTextElement::Latin)
    );
}

#[test]
fn text_declaration_budgets_cancellation_and_real_edit_preserve_bound_metadata() {
    let b = body("<a:bodyPr/><a:p><a:r><a:rPr sz=\"2400\"/><a:t>Before</a:t></a:r></a:p>");
    let p = package(&b);
    for limits in [
        SourceLimits {
            max_text_style_elements: 0,
            ..Default::default()
        },
        SourceLimits {
            max_text_style_attribute_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            inspect_source(&p, limits, &|| false),
            Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
        ));
    }
    assert!(inspect_source(&p, SourceLimits::default(), &|| true).is_err());
    let i = inspect(&b);
    let object = &i.surfaces[SLIDE].objects[0];
    let edits = SourceTextEdits {
        expected_source_sha256: i.source_sha256.clone(),
        edits: vec![SourceTextEdit {
            target: SourceTextTarget {
                part: SLIDE.into(),
                object_id: object.native_id,
                paragraph: 0,
                run: 0,
            },
            expected_text: "Before".into(),
            replacement: "After 中".into(),
        }],
    };
    let output = edit_source_text(&p, &edits, SourceLimits::default(), &|| false).unwrap();
    let after = inspect(&output);
    assert_eq!(after.surfaces[SLIDE].text, i.surfaces[SLIDE].text);
    assert_eq!(
        after.surfaces[SLIDE].objects[0].paragraphs[0][0].text,
        "After 中"
    );
}
