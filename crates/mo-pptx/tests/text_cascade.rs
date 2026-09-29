mod support;
use mo_opc::PartName;
use mo_pptx::{
    source::{
        text::{cascade::*, *},
        *,
    },
    *,
};
#[path = "support/text.rs"]
mod text_support;
use text_support::*;
#[test]
fn office_defaults_and_end_style_do_not_rewrite_author_text() {
    let b = base(
        "",
        &p(&(run("中 العربية fi") + "<a:endParaRPr sz=\"6000\" b=\"1\"/>")),
    );
    let i = index(&b);
    let before = i.clone();
    let result = text(&i);
    let p = &result.paragraphs[0];
    assert_eq!(p.attributes.left_margin, Some(0));
    assert_eq!(p.attributes.indent, Some(0));
    assert_eq!(p.attributes.latin_line_break, Some(false));
    assert_eq!(p.attributes.hanging_punctuation, Some(true));
    assert_eq!(
        p.attributes.default_tab_size.as_ref().unwrap().lexical(),
        "914400"
    );
    assert_eq!(p.origins.len(), 11);
    assert_eq!(p.runs[0].style.attributes.size, Some(1800));
    assert_eq!(p.runs[0].style.attributes.bold, Some(false));
    assert_eq!(p.runs[0].style.attributes.kerning, None);
    assert!(
        !p.runs[0]
            .style
            .origins
            .contains_key(&CharacterProperty::Kerning)
    );
    assert_eq!(p.runs[0].style.attributes.smart_clean, Some(true));
    assert_eq!(p.runs[0].style.attributes.language, None);
    assert_eq!(p.end_style.attributes.size, Some(6000));
    assert_eq!(p.end_style.attributes.bold, Some(true));
    assert_eq!(i, before);
}
#[test]
fn local_run_paragraph_and_matching_level_merge_per_property() {
    let list = r#"<a:defPPr marL="900"><a:defRPr sz="7000"/></a:defPPr><a:lvl1pPr marL="90"/><a:lvl2pPr lvl="7" marL="100" algn="r"><a:defRPr sz="3000" b="1" i="1" lang="zh-CN"/></a:lvl2pPr>"#;
    let ps = r#"<a:p><a:pPr lvl="1" marL="0" algn="ctr"><a:defRPr b="false" sz="3200"/></a:pPr><a:r><a:rPr sz="2800" kern="0" i="false"/><a:t>Text</a:t></a:r></a:p><a:p><a:r><a:t>Next</a:t></a:r></a:p>"#;
    let i = index(&base(list, ps));
    let result = text(&i);
    let p = &result.paragraphs[0];
    let r = &p.runs[0].style;
    assert_eq!(p.attributes.level, Some(1));
    assert_eq!(p.attributes.left_margin, Some(0));
    assert_eq!(p.attributes.alignment, Some(NativeTextAlign::Ctr));
    assert_eq!(r.attributes.size, Some(2800));
    assert_eq!(r.attributes.bold, Some(false));
    assert_eq!(r.attributes.italic, Some(false));
    assert_eq!(r.attributes.kerning, Some(0));
    assert_eq!(r.attributes.language.as_deref(), Some("zh-CN"));
    let origins = &r.origins;
    assert_ne!(
        origins[&CharacterProperty::Size],
        origins[&CharacterProperty::Bold]
    );
    assert_ne!(
        origins[&CharacterProperty::Bold],
        origins[&CharacterProperty::Language]
    );
    assert_eq!(result.paragraphs[1].attributes.left_margin, Some(90));
    assert_eq!(
        result.paragraphs[1].runs[0].style.attributes.size,
        Some(1800)
    );
}
#[test]
fn selected_choices_are_atomic_and_preserve_source_font_and_paint_nodes() {
    let list = r#"<a:lvl1pPr><a:lnSpc><a:spcPct val="123.0000001%"/></a:lnSpc><a:buChar char="•"/><a:tabLst><a:tab pos="100"/></a:tabLst><a:defRPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:latin typeface="+mj-lt"/><a:ea typeface="Source EA"/></a:defRPr></a:lvl1pPr>"#;
    let ps = r#"<a:p><a:pPr><a:buNone/><a:tabLst/></a:pPr><a:r><a:rPr><a:noFill/><a:latin typeface=""/></a:rPr><a:t>Text</a:t></a:r></a:p>"#;
    let i = index(&base(list, ps));
    let result = text(&i);
    let p = &result.paragraphs[0];
    let c = &p.runs[0].style;
    assert_eq!(
        p.declarations[&ParagraphSlot::Bullet].element,
        NativeTextElement::BuNone
    );
    assert!(
        declaration(&i, &p.declarations[&ParagraphSlot::Tabs])
            .unwrap()
            .children
            .is_empty()
    );
    assert_eq!(
        c.declarations[&CharacterSlot::Fill].element,
        NativeTextElement::NoFill
    );
    let SourceTextValue::Font { font } = &declaration(&i, &c.declarations[&CharacterSlot::Latin])
        .unwrap()
        .value
    else {
        panic!()
    };
    assert_eq!(font.typeface, "");
    assert!(c.declarations.contains_key(&CharacterSlot::EastAsian));
    assert!(p.declarations.contains_key(&ParagraphSlot::LineSpacing));
}
#[test]
fn theme_master_and_presentation_defaults_have_distinct_origins() {
    let b = base("", &p(&run("Native")));
    let b = add_theme(
        &b,
        r#"<a:spDef><a:spPr/><a:bodyPr/><a:lstStyle><a:lvl1pPr marR="3"/></a:lstStyle></a:spDef><a:lnDef><a:spPr/><a:bodyPr/><a:lstStyle><a:lvl1pPr indent="2"/></a:lstStyle></a:lnDef><a:txDef><a:spPr/><a:bodyPr/><a:lstStyle><a:lvl1pPr marL="1"><a:defRPr sz="2700"/></a:lvl1pPr></a:lstStyle></a:txDef>"#,
    );
    let b = add_master(
        &b,
        r#"<p:otherStyle><a:lvl1pPr marL="9" rtl="1"><a:defRPr sz="9900" b="1"/></a:lvl1pPr></p:otherStyle>"#,
    );
    let b = add_presentation(
        &b,
        r#"<a:lvl1pPr><a:defRPr i="1" lang="en-US"/></a:lvl1pPr>"#,
    );
    let i = index(&b);
    let result = text(&i);
    let p = &result.paragraphs[0];
    let c = &p.runs[0].style;
    assert_eq!(p.attributes.left_margin, Some(1));
    assert_eq!(p.attributes.indent, Some(2));
    assert_eq!(p.attributes.right_margin, Some(3));
    assert_eq!(p.attributes.right_to_left, Some(true));
    assert_eq!(c.attributes.size, Some(2700));
    assert_eq!(c.attributes.bold, Some(true));
    assert_eq!(c.attributes.italic, Some(true));
    assert!(
        matches!(&c.origins[&CharacterProperty::Size], TextStyleOrigin::Theme { part, .. } if part == THEME)
    );
    assert!(
        matches!(&c.origins[&CharacterProperty::Bold], TextStyleOrigin::Master { part, .. } if part == MASTER)
    );
    assert!(
        matches!(&c.origins[&CharacterProperty::Italic], TextStyleOrigin::Presentation { part, .. } if part == "/ppt/presentation.xml")
    );
}
#[test]
fn placeholder_chain_uses_matching_level_and_never_template_prompt_runs() {
    let b = placeholder(&base("", &p(&run("Slide"))), "body");
    let b = template(
        &b,
        LAYOUT,
        "",
        r#"<a:p><a:pPr marL="12"><a:defRPr sz="3400"/></a:pPr><a:r><a:rPr b="1"/><a:t>Prompt</a:t></a:r></a:p>"#,
    );
    let b = template(
        &b,
        MASTER,
        "",
        r#"<a:p><a:pPr marL="30" marR="40"/><a:r><a:t>Prompt</a:t></a:r></a:p>"#,
    );
    let b = add_master(
        &b,
        r#"<p:titleStyle><a:lvl1pPr><a:defRPr sz="8000"/></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr i="1"/></a:lvl1pPr></p:bodyStyle>"#,
    );
    let i = index(&b);
    let result = text(&i);
    let p = &result.paragraphs[0];
    assert_eq!(p.attributes.left_margin, Some(12));
    assert_eq!(p.attributes.right_margin, Some(40));
    assert_eq!(p.runs[0].style.attributes.size, Some(3400));
    assert_eq!(p.runs[0].style.attributes.bold, Some(false));
    assert_eq!(p.runs[0].style.attributes.italic, Some(true));
    assert!(
        matches!(&p.origins[&ParagraphProperty::LeftMargin], TextStyleOrigin::Object { object, .. } if object.part == LAYOUT)
    );
    assert!(
        matches!(&p.origins[&ParagraphProperty::RightMargin], TextStyleOrigin::Object { object, .. } if object.part == MASTER)
    );
}
#[test]
fn breaks_fields_and_empty_paragraphs_keep_separate_source_bindings() {
    let ps = r#"<a:p><a:pPr><a:defRPr sz="2400"/></a:pPr><a:r><a:t>A</a:t></a:r><a:br><a:rPr sz="2500"/></a:br><a:fld id="{00000000-0000-0000-0000-000000000001}" type="slidenum"><a:rPr sz="2600"/><a:t>1</a:t></a:fld><a:endParaRPr sz="2700"/></a:p><a:p><a:endParaRPr sz="2800"/></a:p>"#;
    let i = index(&base("", ps));
    let result = text(&i);
    let p = &result.paragraphs[0];
    assert_eq!(
        p.runs.iter().map(|r| r.kind).collect::<Vec<_>>(),
        vec![
            SourceRunKind::Text,
            SourceRunKind::Break,
            SourceRunKind::Field
        ]
    );
    assert_eq!(
        p.runs
            .iter()
            .map(|r| r.style.attributes.size)
            .collect::<Vec<_>>(),
        vec![Some(2400), Some(2500), Some(2600)]
    );
    assert_eq!(p.end_style.attributes.size, Some(2700));
    assert!(result.paragraphs[1].runs.is_empty());
    assert_eq!(result.paragraphs[1].end_style.attributes.size, Some(2800));
    assert!(
        p.runs
            .windows(2)
            .all(|r| r[0].source_ordinal < r[1].source_ordinal)
    );
}
#[test]
fn unknown_content_is_not_silently_promoted_to_a_resolved_style() {
    for ps in [
        r#"<a:p><a:pPr mystery="1"/><a:r><a:t>T</a:t></a:r></a:p>"#,
        r#"<a:p><a:r><a:t xml:space="default"> T </a:t></a:r></a:p>"#,
        r#"<a:p><a:r><a:t xml:space="unknown"> T </a:t></a:r></a:p>"#,
        r#"<a:p><a:pPr><a:buBlip/></a:pPr><a:r><a:t>T</a:t></a:r></a:p>"#,
    ] {
        let i = index(&base("", ps));
        assert!(matches!(
            outcome(&i),
            TextCascadeOutcome::Unresolved {
                reason: TextCascadeUnresolved::RetainedContent { .. }
            }
        ));
    }
    let i = index(&base(r#"<a:defPPr mystery="1"/>"#, &p(&run("T"))));
    assert!(matches!(outcome(&i), TextCascadeOutcome::Cascaded { .. }));
    let i = index(&base(
        "",
        r#"<a:p><a:fld id="{00000000-0000-0000-0000-000000000001}"><a:pPr/><a:t>F</a:t></a:fld></a:p>"#,
    ));
    assert!(matches!(
        outcome(&i),
        TextCascadeOutcome::Unresolved {
            reason: TextCascadeUnresolved::FieldParagraph { .. }
        }
    ));
}
#[test]
fn explicit_space_preservation_keeps_exact_text_and_source_edit_binding() {
    let i = index(&base(
        "",
        r#"<a:p><a:r><a:t xml:space="preserve">  中 &amp; A  </a:t></a:r></a:p>"#,
    ));
    let before = i.clone();
    assert!(matches!(outcome(&i), TextCascadeOutcome::Cascaded { .. }));
    let object = i
        .surfaces
        .values()
        .flat_map(|surface| &surface.objects)
        .find(|object| {
            object
                .paragraphs
                .iter()
                .flatten()
                .any(|run| run.text == "  中 & A  ")
        })
        .unwrap();
    assert!(object.paragraphs[0][0].editable);
    assert_eq!(i, before);
}
#[test]
fn ambiguous_placeholders_and_template_levels_require_resolution() {
    let b = placeholder(&base("", &p(&run("T"))), "body");
    let i = index(&b);
    assert!(matches!(
        outcome(&i),
        TextCascadeOutcome::Unresolved {
            reason: TextCascadeUnresolved::Placeholder { .. }
        }
    ));
    let b = template(
        &b,
        LAYOUT,
        "",
        "<a:p><a:pPr/><a:endParaRPr/></a:p><a:p><a:pPr/><a:endParaRPr/></a:p>",
    );
    let b = template(&b, MASTER, "", "<a:p/>");
    let i = index(&b);
    assert!(matches!(
        outcome(&i),
        TextCascadeOutcome::Unresolved {
            reason: TextCascadeUnresolved::AmbiguousTemplateParagraph { .. }
        }
    ));
}
#[test]
fn invalid_bindings_cancellation_and_all_budgets_abort_without_partial_success() {
    let i = index(&base("", &p(&(run("A") + &run("B")))));
    let t = target(&i);
    assert!(
        resolve(
            &i,
            &i.source_sha256,
            &t,
            TextCascadeLimits::default(),
            &|| true
        )
        .is_err()
    );
    let wrong = mo_common::Digest::from_sha256([0; 32]);
    assert!(resolve(&i, &wrong, &t, TextCascadeLimits::default(), &|| false).is_err());
    for limits in [
        TextCascadeLimits {
            max_paragraphs: 0,
            ..Default::default()
        },
        TextCascadeLimits {
            max_runs: 1,
            ..Default::default()
        },
        TextCascadeLimits {
            max_steps: 1,
            ..Default::default()
        },
        TextCascadeLimits {
            max_lexical_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            resolve(&i, &i.source_sha256, &t, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    let mut damaged = i.clone();
    damaged.surfaces.get_mut(SLIDE).unwrap().objects[0].paragraphs[0].clear();
    assert!(
        resolve(
            &damaged,
            &damaged.source_sha256,
            &t,
            TextCascadeLimits::default(),
            &|| false
        )
        .is_err()
    );
    let mut damaged = i.clone();
    let s = damaged.surfaces.get_mut(SLIDE).unwrap();
    let id = s.objects[0].text_body_ordinal.unwrap();
    s.text.nodes.get_mut(&id).unwrap().children.push(id);
    assert!(
        resolve(
            &damaged,
            &damaged.source_sha256,
            &t,
            TextCascadeLimits::default(),
            &|| false
        )
        .is_err()
    );
}
#[test]
fn actual_package_changes_refresh_cascade_without_changing_text() {
    let b = base("", &p(&run("Native source")));
    let before = index(&b);
    let b = rewrite(&b, SLIDE, |s| {
        s.replacen("<a:p>", "<a:p><a:pPr><a:defRPr sz=\"3600\"/></a:pPr>", 1)
    });
    let after = index(&b);
    assert_ne!(before.source_sha256, after.source_sha256);
    assert_eq!(
        before.surfaces[SLIDE].objects[0].paragraphs,
        after.surfaces[SLIDE].objects[0].paragraphs
    );
    assert_eq!(
        text(&before).paragraphs[0].runs[0].style.attributes.size,
        Some(1800)
    );
    assert_eq!(
        text(&after).paragraphs[0].runs[0].style.attributes.size,
        Some(3600)
    );
}

#[test]
fn all_nine_levels_remain_independent_across_repeated_paragraphs() {
    let mut list = String::new();
    let mut paragraphs = String::new();
    for level in 0..9 {
        let n = level + 1;
        list += &format!(
            "<a:lvl{n}pPr lvl=\"0\" marL=\"{n}\"><a:defRPr sz=\"{}\"/></a:lvl{n}pPr>",
            1800 + level * 100
        );
    }
    for level in (0..9).cycle().take(180) {
        paragraphs +=
            &format!("<a:p><a:pPr lvl=\"{level}\"/><a:r><a:t>Level {level}</a:t></a:r></a:p>");
    }
    let i = index(&base(&list, &paragraphs));
    let result = text(&i);
    assert_eq!(result.paragraphs.len(), 180);
    for (position, p) in result.paragraphs.iter().enumerate() {
        let level = (position % 9) as i32;
        assert_eq!(p.attributes.level, Some(level));
        assert_eq!(p.attributes.left_margin, Some(level + 1));
        assert_eq!(p.runs[0].style.attributes.size, Some(1800 + level * 100));
        if position >= 9 {
            assert_eq!(
                p.runs[0].style,
                result.paragraphs[position % 9].runs[0].style
            );
        }
    }
}

#[test]
fn title_center_title_body_and_other_use_their_own_master_style() {
    for (kind, size) in [
        ("title", 4000),
        ("ctrTitle", 4000),
        ("body", 3000),
        ("subTitle", 3000),
        ("obj", 3000),
    ] {
        let b = placeholder(&base("", &p(&run("T"))), kind);
        let b = template(&b, LAYOUT, "", "<a:p/>");
        let b = template(&b, MASTER, "", "<a:p/>");
        let b = add_master(
            &b,
            "<p:titleStyle><a:lvl1pPr><a:defRPr sz=\"4000\"/></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr sz=\"3000\"/></a:lvl1pPr></p:bodyStyle><p:otherStyle><a:lvl1pPr><a:defRPr sz=\"2000\"/></a:lvl1pPr></p:otherStyle>",
        );
        let i = index(&b);
        assert_eq!(
            text(&i).paragraphs[0].runs[0].style.attributes.size,
            Some(size),
            "{kind}"
        );
    }
    let b = add_master(
        &base("", &p(&run("T"))),
        "<p:otherStyle><a:lvl1pPr><a:defRPr sz=\"2000\"/></a:lvl1pPr></p:otherStyle>",
    );
    let i = index(&b);
    assert_eq!(
        text(&i).paragraphs[0].runs[0].style.attributes.size,
        Some(2000)
    );
}

#[test]
fn selected_mce_branch_retains_physical_origins_and_cancellation_is_polled() {
    let b = base(
        "",
        "<a:p><a:r><a:rPr sz=\"2200\"/><a:t>Branch</a:t></a:r></a:p>",
    );
    let b = rewrite(&b, SLIDE, |s| {
        s.replacen("<a:rPr sz=\"2200\"/>", "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:u=\"urn:unknown\"><mc:Choice Requires=\"u\"><a:rPr sz=\"9900\"/></mc:Choice><mc:Fallback><a:rPr sz=\"2200\"/></mc:Fallback></mc:AlternateContent>", 1)
    });
    let i = index(&b);
    let result = text(&i);
    let c = &result.paragraphs[0].runs[0].style;
    assert_eq!(c.attributes.size, Some(2200));
    let TextStyleOrigin::Object { source_ordinal, .. } = c.origins[&CharacterProperty::Size] else {
        panic!()
    };
    // Count physical XML element starts independently of the source catalog.
    let xml = String::from_utf8(
        package(&b)
            .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
            .unwrap(),
    )
    .unwrap();
    let before = xml.split("<a:rPr sz=\"2200\"/>").next().unwrap();
    let starts = before
        .split('<')
        .skip(1)
        .filter(|s| !s.starts_with('/') && !s.starts_with('?') && !s.starts_with('!'))
        .count();
    assert_eq!(source_ordinal as usize, starts);
    let calls = std::cell::Cell::new(0);
    let r = resolve(
        &i,
        &i.source_sha256,
        &target(&i),
        TextCascadeLimits::default(),
        &|| {
            calls.set(calls.get() + 1);
            calls.get() == 20
        },
    );
    assert!(matches!(r, Err(PptxError::Cancelled)));
    assert_eq!(calls.get(), 20);
}
