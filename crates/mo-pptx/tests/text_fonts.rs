mod support;
#[allow(dead_code)]
#[path = "support/text.rs"]
mod text_support;
use mo_pptx::{
    source::{
        SourceLimits, inspect_source,
        text::{cascade, fonts::*},
    },
    *,
};
use text_support::*;

fn body_font(tag: &str, name: &str) -> Vec<u8> {
    base(
        "",
        &format!(
            "<a:p><a:r><a:rPr><a:{tag} typeface=\"{name}\"/></a:rPr><a:t>Native font</a:t></a:r></a:p>"
        ),
    )
}
fn scheme(b: &[u8], ea: &str, cs: &str, supplements: &str) -> Vec<u8> {
    rewrite(b, THEME, |mut s| {
        let a = s.find("<a:fontScheme").unwrap();
        let z = a + s[a..].find("</a:fontScheme>").unwrap() + "</a:fontScheme>".len();
        let mut xml = "<a:fontScheme name=\"Owned\">".to_owned();
        for (group, latin) in [("majorFont", "Major Latin"), ("minorFont", "Minor Latin")] {
            xml += &format!(
                "<a:{group}><a:latin typeface=\"{latin}\"/><a:ea typeface=\"{ea}\"/><a:cs typeface=\"{cs}\"/>{supplements}</a:{group}>"
            );
        }
        xml += "</a:fontScheme>";
        s.replace_range(a..z, &xml);
        s
    })
}
fn reference(idx: &str) -> String {
    format!(
        "<a:lnRef idx=\"0\"><a:schemeClr val=\"tx1\"/></a:lnRef><a:fillRef idx=\"0\"><a:schemeClr val=\"tx1\"/></a:fillRef><a:effectRef idx=\"0\"><a:schemeClr val=\"tx1\"/></a:effectRef><a:fontRef idx=\"{idx}\"><a:schemeClr val=\"tx1\"/></a:fontRef>"
    )
}
fn shape_reference(b: &[u8], part: &str, idx: &str) -> Vec<u8> {
    rewrite(b, part, |s| {
        // template() appends the matching owned placeholder after existing
        // shapes. The slide target is the first shape from base().
        let start = if part == SLIDE {
            0
        } else {
            s.rfind("<p:sp>").unwrap()
        };
        s[..start].to_owned()
            + &s[start..].replacen(
                "</p:spPr>",
                &format!("</p:spPr><p:style>{}</p:style>", reference(idx)),
                1,
            )
    })
}
fn font(b: &[u8], slot: NativeFontSlot, script: Option<&str>) -> TypefaceOutcome {
    let i = index(b);
    let text = text(&i);
    let r = resolve(
        &i,
        &text,
        0,
        Some(0),
        slot,
        script,
        TypefaceLimits::default(),
        &|| false,
    )
    .unwrap();
    if let Some(dir) = std::env::var_os("MO_NATIVE_FONT_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        let name = format!(
            "{}-{slot:?}-{}",
            i.source_sha256.as_str(),
            script.unwrap_or("none")
        );
        std::fs::write(dir.join(format!("{name}.pptx")), b).unwrap();
        std::fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(
                &serde_json::json!({"slot":slot,"script":script,"cascade":text,"font":r}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    r
}
fn named(b: &[u8], slot: NativeFontSlot, script: Option<&str>) -> NativeTypeface {
    match font(b, slot, script) {
        TypefaceOutcome::Named { font } => *font,
        other => panic!("{other:?}"),
    }
}
#[test]
fn direct_fonts_and_all_six_theme_tokens_preserve_their_source() {
    let f = named(
        &body_font("latin", "MusterOffice Synthetic"),
        NativeFontSlot::Latin,
        None,
    );
    assert_eq!(f.typeface, "MusterOffice Synthetic");
    assert!(f.theme.is_none());
    assert_eq!(f.authored_font.unwrap().typeface, "MusterOffice Synthetic");
    for (tag, slot, suffix, name) in [
        ("latin", NativeFontSlot::Latin, "lt", "Latin"),
        ("ea", NativeFontSlot::EastAsian, "ea", "EA"),
        ("cs", NativeFontSlot::ComplexScript, "cs", "CS"),
    ] {
        for (prefix, latin) in [("mj", "Major"), ("mn", "Minor")] {
            let token = format!("+{prefix}-{suffix}");
            let b = scheme(&body_font(tag, &token), "EA", "CS", "");
            let f = named(&b, slot, None);
            assert_eq!(
                f.typeface,
                if name == "Latin" {
                    format!("{latin} Latin")
                } else {
                    name.into()
                }
            );
            assert_eq!(f.authored_font.as_ref().unwrap().typeface, token);
            assert_eq!(f.theme.as_ref().unwrap().scheme.part, THEME);
            assert_eq!(f.theme.as_ref().unwrap().slot, slot);
            assert!(f.theme.unwrap().supplemental.is_none());
        }
    }
}
#[test]
fn empty_theme_slots_use_explicit_script_keys_and_never_first_font_guessing() {
    let supplements = "<a:font script=\"Hans\" typeface=\"Simplified\"/><a:font script=\"Hant\" typeface=\"Traditional\"/><a:font script=\"Arab\" typeface=\"Arabic\"/>";
    let b = scheme(&body_font("ea", "+mn-ea"), "", "", supplements);
    for (script, expected, position) in [("Hans", "Simplified", 0), ("Hant", "Traditional", 1)] {
        let f = named(&b, NativeFontSlot::EastAsian, Some(script));
        assert_eq!(f.typeface, expected);
        assert_eq!(f.theme.unwrap().supplemental, Some(position));
    }
    assert!(matches!(
        font(&b, NativeFontSlot::EastAsian, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::ScriptRequired {}
        }
    ));
    assert!(matches!(
        font(&b, NativeFontSlot::EastAsian, Some("Jpan")),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::MissingSupplemental { .. }
        }
    ));
    let b = scheme(&body_font("cs", "+mn-cs"), "", "", supplements);
    assert_eq!(
        named(&b, NativeFontSlot::ComplexScript, Some("Arab")).typeface,
        "Arabic"
    );
    let b = scheme(&body_font("ea", "+mn-ea"), "Declared EA", "", supplements);
    assert_eq!(
        named(&b, NativeFontSlot::EastAsian, Some("Hans")).typeface,
        "Declared EA"
    );
}
#[test]
fn font_ref_uses_shape_then_theme_defaults_and_explicit_fonts_override_it() {
    let b = scheme(
        &base("", "<a:p><a:r><a:t>Native</a:t></a:r></a:p>"),
        "EA",
        "CS",
        "",
    );
    let b = add_theme(
        &b,
        &format!(
            "<a:txDef><a:spPr/><a:bodyPr/><a:lstStyle/><a:style>{}</a:style></a:txDef>",
            reference("major")
        ),
    );
    let f = named(&b, NativeFontSlot::Latin, None);
    assert_eq!(f.typeface, "Major Latin");
    assert!(matches!(
        f.declared_by.origin,
        cascade::TextStyleOrigin::Theme { .. }
    ));
    let b = shape_reference(&b, SLIDE, "minor");
    let f = named(&b, NativeFontSlot::Latin, None);
    assert_eq!(f.typeface, "Minor Latin");
    assert!(matches!(
        f.declared_by.origin,
        cascade::TextStyleOrigin::Object { .. }
    ));
    let b = rewrite(&b, SLIDE, |s| {
        s.replacen(
            "<a:r><a:t>",
            "<a:r><a:rPr><a:latin typeface=\"Explicit\"/></a:rPr><a:t>",
            1,
        )
    });
    assert_eq!(named(&b, NativeFontSlot::Latin, None).typeface, "Explicit");
}
#[test]
fn default_style_capture_is_scoped_and_unknown_font_metadata_is_not_ignored() {
    let b = base("", "<a:p><a:r><a:t>Native</a:t></a:r></a:p>");
    let b = add_theme(
        &b,
        &format!(
            "<a:txDef><a:spPr><a:fontRef idx=\"invalid\"/></a:spPr><a:bodyPr/><a:lstStyle/><a:style>{}</a:style></a:txDef>",
            reference("minor")
        ),
    );
    assert!(matches!(
        font(&b, NativeFontSlot::Latin, None),
        TypefaceOutcome::Named { .. }
    ));
    let b = scheme(&body_font("latin", "+mj-lt"), "EA", "CS", "");
    let b = rewrite(&b, THEME, |s| {
        s.replacen("<a:majorFont>", "<a:majorFont unknown=\"1\">", 1)
    });
    assert!(matches!(
        font(&b, NativeFontSlot::Latin, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::RetainedTheme { .. }
        }
    ));
    let b = scheme(&body_font("latin", "+mj-lt"), "EA", "CS", "");
    let b = rewrite(&b, THEME, |s| {
        s.replacen("<a:majorFont>", "<a:majorFont xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:u=\"urn:owned:font\" mc:Ignorable=\"u\"><u:metadata/>", 1)
    });
    // Ignorable elements are removed by MCE; the native font semantics are unchanged.
    assert_eq!(
        named(&b, NativeFontSlot::Latin, None).typeface,
        "Major Latin"
    );
}
#[test]
fn duplicate_supplements_empty_names_and_none_references_remain_diagnostics() {
    let b = scheme(
        &body_font("ea", "+mn-ea"),
        "",
        "",
        "<a:font script=\"Hans\" typeface=\"A\"/><a:font script=\"Hans\" typeface=\"B\"/>",
    );
    assert!(matches!(
        font(&b, NativeFontSlot::EastAsian, Some("Hans")),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::AmbiguousSupplemental { .. }
        }
    ));
    assert!(matches!(
        font(&body_font("latin", ""), NativeFontSlot::Latin, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::EmptyTypeface {}
        }
    ));
    assert!(matches!(
        font(
            &body_font("latin", "+mj-unknown"),
            NativeFontSlot::Latin,
            None
        ),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::UnknownThemeToken { .. }
        }
    ));
    let b = shape_reference(
        &base("", "<a:p><a:r><a:t>Native</a:t></a:r></a:p>"),
        SLIDE,
        "none",
    );
    assert!(matches!(
        font(&b, NativeFontSlot::Latin, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::DisabledThemeFont {}
        }
    ));
}
#[test]
fn stale_binding_bad_script_limits_and_cancellation_do_not_resolve_fonts() {
    let b = scheme(
        &body_font("ea", "+mn-ea"),
        "",
        "",
        "<a:font script=\"Hans\" typeface=\"A\"/>",
    );
    let i = index(&b);
    let text = text(&i);
    assert!(
        resolve(
            &i,
            &text,
            0,
            Some(0),
            NativeFontSlot::EastAsian,
            Some("zh-CN"),
            TypefaceLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert!(
        resolve(
            &i,
            &text,
            0,
            Some(0),
            NativeFontSlot::EastAsian,
            Some("Hans"),
            TypefaceLimits {
                max_supplements: 0,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert!(
        resolve(
            &i,
            &text,
            0,
            Some(0),
            NativeFontSlot::EastAsian,
            Some("Hans"),
            TypefaceLimits {
                max_name_bytes: 1,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert!(matches!(
        resolve(
            &i,
            &text,
            0,
            Some(0),
            NativeFontSlot::EastAsian,
            Some("Hans"),
            TypefaceLimits::default(),
            &|| true
        ),
        Err(PptxError::Cancelled)
    ));
    let mut stale = text.clone();
    stale.source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(
        resolve(
            &i,
            &stale,
            0,
            Some(0),
            NativeFontSlot::EastAsian,
            Some("Hans"),
            TypefaceLimits::default(),
            &|| false
        )
        .is_err()
    );
}

#[test]
fn actual_theme_override_replaces_the_font_scheme_as_a_whole() {
    use mo_opc::{PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource};
    let b = scheme(&body_font("latin", "+mj-lt"), "EA", "CS", "");
    let source = package(&b);
    let mut builder = PackageBuilder::new();
    for (part, info) in source.parts() {
        if mo_opc::relationship_source(part).unwrap().is_none() {
            builder
                .add_part(
                    part.clone(),
                    info.content_type.clone(),
                    source.read_part(part, 1 << 20, &|| false).unwrap(),
                )
                .unwrap();
        }
    }
    let part = PartName::new("/ppt/theme/fontOverride.xml").unwrap();
    let xml = "<a:themeOverride xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:fontScheme name=\"Override\"><a:majorFont><a:latin typeface=\"Override Major\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"Override Minor\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme></a:themeOverride>";
    builder
        .add_part(
            part.clone(),
            "application/vnd.openxmlformats-officedocument.themeOverride+xml".into(),
            xml.as_bytes(),
        )
        .unwrap();
    for (owner, relationships) in source.relationships() {
        let mut relationships = relationships.clone();
        if owner == &RelationshipSource::Part(PartName::new(SLIDE).unwrap()) {
            relationships.push(Relationship::new(owner, "rIdFontOverride".into(), "http://schemas.openxmlformats.org/officeDocument/2006/relationships/themeOverride".into(), "../theme/fontOverride.xml".into(), false).unwrap());
        }
        builder
            .set_relationships(owner.clone(), relationships)
            .unwrap();
    }
    let b = builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    let f = named(&b, NativeFontSlot::Latin, None);
    assert_eq!(f.typeface, "Override Major");
    assert_eq!(f.theme.unwrap().scheme.part, part.to_string());
}

#[test]
fn malformed_and_retained_default_font_styles_do_not_synthesize_success() {
    for style in [
        "<a:style/>",
        "<a:style><a:fontRef idx=\"minor\"/><a:fontRef idx=\"major\"/></a:style>",
    ] {
        let b = add_theme(
            &base("", "<a:p><a:r><a:t>T</a:t></a:r></a:p>"),
            &format!("<a:txDef><a:spPr/><a:bodyPr/><a:lstStyle/>{style}</a:txDef>"),
        );
        assert!(inspect_source(&package(&b), SourceLimits::default(), &|| false).is_err());
    }
    let b = add_theme(
        &base("", "<a:p><a:r><a:t>T</a:t></a:r></a:p>"),
        &format!(
            "<a:txDef><a:spPr/><a:bodyPr/><a:lstStyle/><a:style unknown=\"1\">{}</a:style></a:txDef>",
            reference("minor")
        ),
    );
    let i = index(&b);
    assert!(matches!(
        outcome(&i),
        cascade::TextCascadeOutcome::Unresolved {
            reason: cascade::TextCascadeUnresolved::RetainedContent { .. }
        }
    ));
}

#[test]
fn font_ref_follows_matched_placeholders_and_explicit_default_precedence() {
    let b = scheme(
        &placeholder(&base("", "<a:p><a:r><a:t>T</a:t></a:r></a:p>"), "body"),
        "EA",
        "CS",
        "",
    );
    let b = template(&b, LAYOUT, "", "<a:p/>");
    let b = template(&b, MASTER, "", "<a:p/>");
    let b = shape_reference(&b, MASTER, "minor");
    let f = named(&b, NativeFontSlot::Latin, None);
    assert_eq!(f.typeface, "Minor Latin");
    assert!(
        matches!(f.declared_by.origin, cascade::TextStyleOrigin::Object { object, .. } if object.part == MASTER)
    );
    let b = add_theme(
        &b,
        &format!(
            "<a:txDef><a:spPr/><a:bodyPr/><a:lstStyle/><a:style>{}</a:style></a:txDef>",
            reference("major")
        ),
    );
    let f = named(&b, NativeFontSlot::Latin, None);
    assert_eq!(f.typeface, "Major Latin");
    assert!(matches!(
        f.declared_by.origin,
        cascade::TextStyleOrigin::Theme { .. }
    ));
    let b = shape_reference(&b, LAYOUT, "minor");
    let f = named(&b, NativeFontSlot::Latin, None);
    assert_eq!(f.typeface, "Minor Latin");
    assert!(
        matches!(f.declared_by.origin, cascade::TextStyleOrigin::Object { object, .. } if object.part == LAYOUT)
    );
}

#[test]
fn font_metadata_is_preserved_and_retained_declarations_are_diagnostic() {
    let b = scheme(&body_font("latin", "+mj-lt"), "EA", "CS", "");
    let b = rewrite(&b, SLIDE, |s| {
        s.replace("typeface=\"+mj-lt\"", "typeface=\"+mj-lt\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"-122\"")
    });
    let b = rewrite(&b, THEME, |s| {
        s.replace(
            "typeface=\"Major Latin\"",
            "typeface=\"Major Latin\" pitchFamily=\"18\" charset=\"0\"",
        )
    });
    let i = index(&b);
    let f = named(&b, NativeFontSlot::Latin, None);
    let node = cascade::declaration(&i, &f.declared_by).unwrap();
    let mo_pptx::source::text::SourceTextValue::Font { font: original } = &node.value else {
        panic!("font")
    };
    assert_eq!(f.authored_font.as_ref(), Some(original.as_ref()));
    assert_ne!(f.authored_font, f.theme_font);
    let b = rewrite(&b, SLIDE, |s| {
        s.replace("typeface=\"+mj-lt\"", "typeface=\"+mj-lt\" unknown=\"1\"")
    });
    assert!(matches!(
        outcome(&index(&b)),
        cascade::TextCascadeOutcome::Unresolved {
            reason: cascade::TextCascadeUnresolved::RetainedContent { .. }
        }
    ));
    let b = shape_reference(
        &base("", "<a:p><a:r><a:t>T</a:t></a:r></a:p>"),
        SLIDE,
        "major",
    );
    assert!(matches!(
        font(&b, NativeFontSlot::Symbol, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::SymbolThemeFont {}
        }
    ));
    assert!(matches!(
        font(
            &base("", "<a:p><a:r><a:t>T</a:t></a:r></a:p>"),
            NativeFontSlot::Latin,
            None
        ),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::MissingDeclaration {}
        }
    ));
}
