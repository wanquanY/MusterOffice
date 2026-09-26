mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{fill::*, line::SourceLineFill, *},
    *,
};

const SLIDE: &str = "/ppt/slides/slide1.xml";
const GRADIENT: &str = r#"<a:gradFill flip="xy" rotWithShape="0"><a:gsLst><a:gs pos="+0050000"><a:schemeClr val="accent1"><a:alpha val="50%"/><a:alpha val="25000"/></a:schemeClr></a:gs><a:gs pos="50.00%"><a:srgbClr val="001122"/></a:gs></a:gsLst><a:path path="circle"><a:fillToRect l="-25.00000001%" r="150000"/></a:path><a:tileRect t="0"/></a:gradFill>"#;
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
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
fn rewrite(bytes: &[u8], part: &str, change: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(bytes);
    let name = PartName::new(part).unwrap();
    let xml = String::from_utf8(p.read_part(&name, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(name, change(xml).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn fixture(fill: &str, style: &str) -> Vec<u8> {
    rewrite(&base(), SLIDE, |mut xml| {
        let first = xml.find("<p:sp>").unwrap();
        let a = first + xml[first..].find("<p:spPr>").unwrap();
        let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
        xml.replace_range(a..b, &format!("<p:spPr>{fill}</p:spPr>{style}"));
        xml
    })
}
fn inspect(bytes: &[u8]) -> Result<SourceIndex, PptxError> {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false)
}
fn object(index: &SourceIndex) -> &SourceObject {
    &index.surfaces[SLIDE].objects[0]
}

#[test]
fn absent_empty_explicit_zero_and_no_fill_are_distinct() {
    assert!(object(&inspect(&fixture("", "")).unwrap()).fill.is_none());
    let empty = inspect(&fixture("<a:solidFill/>", "")).unwrap();
    assert!(matches!(
        object(&empty).fill.as_ref().unwrap().definition,
        SourceFillDefinition::Solid { color: None }
    ));
    let empty = inspect(&fixture("<a:gradFill/>", "")).unwrap();
    let SourceFillDefinition::Gradient(g) = &object(&empty).fill.as_ref().unwrap().definition
    else {
        panic!()
    };
    assert!(
        g.stops.is_none() && g.shade.is_none() && g.flip.is_none() && g.rotate_with_shape.is_none()
    );
    let zero = inspect(&fixture(r#"<a:gradFill rotWithShape="false" flip="none"><a:lin ang="0" scaled="0"/><a:tileRect l="0"/></a:gradFill>"#, "")).unwrap();
    let SourceFillDefinition::Gradient(g) = &object(&zero).fill.as_ref().unwrap().definition else {
        panic!()
    };
    assert_eq!(g.rotate_with_shape, Some(false));
    assert!(matches!(
        g.shade,
        Some(SourceGradientShade::Linear {
            angle: Some(0),
            scaled: Some(false),
            ..
        })
    ));
    assert_eq!(
        g.tile_rect
            .as_ref()
            .unwrap()
            .left
            .as_ref()
            .unwrap()
            .lexical(),
        "0"
    );
    for (xml, name) in [("<a:noFill/>", "none"), ("<a:grpFill/>", "group")] {
        let index = inspect(&fixture(xml, "")).unwrap();
        assert_eq!(
            serde_json::to_value(object(&index).fill.as_ref().unwrap()).unwrap()["definition"]["kind"],
            name
        );
    }
}

#[test]
fn gradient_preserves_order_equal_positions_color_transforms_and_rect_precision() {
    let index = inspect(&fixture(GRADIENT, "")).unwrap();
    let f = object(&index).fill.as_ref().unwrap();
    let SourceFillDefinition::Gradient(g) = &f.definition else {
        panic!()
    };
    let stops = g.stops.as_ref().unwrap();
    assert_eq!(stops.entries.len(), 2);
    assert_eq!(stops.entries[0].position.lexical(), "+0050000");
    assert_eq!(stops.entries[1].position.lexical(), "50.00%");
    assert_eq!(stops.entries[0].color.transforms.len(), 2);
    assert!(
        f.source_ordinal < stops.source_ordinal
            && stops.source_ordinal < stops.entries[0].source_ordinal
            && stops.entries[0].source_ordinal < stops.entries[0].color.source_ordinal
            && stops.entries[0].color.source_ordinal < stops.entries[1].source_ordinal
    );
    let Some(SourceGradientShade::Path {
        fill_to_rect: Some(r),
        ..
    }) = &g.shade
    else {
        panic!()
    };
    assert_eq!(r.left.as_ref().unwrap().lexical(), "-25.00000001%");
    assert_eq!(r.right.as_ref().unwrap().lexical(), "150000");
    assert!(f.retained_ordinals.is_empty());
}

#[test]
fn pattern_colors_and_all_54_native_patterns_are_typed() {
    let patterns = "pct5 pct10 pct20 pct25 pct30 pct40 pct50 pct60 pct70 pct75 pct80 pct90 horz vert ltHorz ltVert dkHorz dkVert narHorz narVert dashHorz dashVert cross dnDiag upDiag ltDnDiag ltUpDiag dkDnDiag dkUpDiag wdDnDiag wdUpDiag dashDnDiag dashUpDiag diagCross smCheck lgCheck smGrid lgGrid dotGrid smConfetti lgConfetti horzBrick diagBrick solidDmnd openDmnd dotDmnd plaid sphere weave divot shingle wave trellis zigZag";
    assert_eq!(patterns.split_whitespace().count(), 54);
    for pattern in patterns.split_whitespace() {
        let index = inspect(&fixture(&format!(r#"<a:pattFill prst="{pattern}"><a:fgClr><a:prstClr val="red"/></a:fgClr><a:bgClr><a:sysClr val="window" lastClr="F0F0F0"/></a:bgClr></a:pattFill>"#), "")).unwrap();
        let SourceFillDefinition::Pattern(p) = &object(&index).fill.as_ref().unwrap().definition
        else {
            panic!()
        };
        assert_eq!(serde_json::to_value(p.preset).unwrap(), pattern);
        assert!(p.foreground.is_some() && p.background.is_some());
    }
}

#[test]
fn image_relationships_crop_tile_and_units_remain_source_declarations() {
    let xml = r#"<a:blipFill dpi="0" rotWithShape="1"><a:blip r:embed="ownedImage" r:link="ownedLink" cstate="hqprint"/><a:srcRect l="-5.123456789%"/><a:tile tx="-0.000000001cm" ty="+00127" sx="0" sy="-20%" flip="y" algn="br"/></a:blipFill>"#;
    let index = inspect(&fixture(xml, "")).unwrap();
    let SourceFillDefinition::Image(i) = &object(&index).fill.as_ref().unwrap().definition else {
        panic!()
    };
    assert_eq!(i.dpi, Some(0));
    assert_eq!(i.rotate_with_shape, Some(true));
    assert_eq!(
        i.blip.as_ref().unwrap().embed.as_deref(),
        Some("ownedImage")
    );
    assert_eq!(i.blip.as_ref().unwrap().link.as_deref(), Some("ownedLink"));
    assert_eq!(
        i.source_rect
            .as_ref()
            .unwrap()
            .left
            .as_ref()
            .unwrap()
            .lexical(),
        "-5.123456789%"
    );
    let Some(SourceImageFillMode::Tile(t)) = &i.mode else {
        panic!()
    };
    assert_eq!(t.translate_x.as_ref().unwrap().lexical(), "-0.000000001cm");
    assert_eq!(t.translate_y.as_ref().unwrap().lexical(), "+00127");
    assert_eq!(t.scale_y.as_ref().unwrap().lexical(), "-20%");
    assert!(
        object(&index)
            .fill
            .as_ref()
            .unwrap()
            .retained_ordinals
            .is_empty()
    );
    let stretch = inspect(&fixture(
        "<a:blipFill><a:stretch><a:fillRect b=\"0\"/></a:stretch></a:blipFill>",
        "",
    ))
    .unwrap();
    let SourceFillDefinition::Image(i) = &object(&stretch).fill.as_ref().unwrap().definition else {
        panic!()
    };
    assert!(matches!(
        i.mode,
        Some(SourceImageFillMode::Stretch {
            fill_rect: Some(_),
            ..
        })
    ));
}

#[test]
fn pictures_groups_root_group_and_style_references_use_separate_bindings() {
    let bytes = rewrite(
        &fixture(
            "<a:grpFill/>",
            r#"<p:style><a:fillRef idx="1001"><a:schemeClr val="accent2"/></a:fillRef></p:style>"#,
        ),
        SLIDE,
        |xml| {
            xml.replace("<p:grpSpPr/>", "<p:grpSpPr><a:solidFill><a:srgbClr val=\"AA0000\"/></a:solidFill></p:grpSpPr>")
            .replace("</p:spTree>", "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"900\" name=\"owned\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:pattFill prst=\"cross\"/></p:grpSpPr></p:grpSp></p:spTree>")
        },
    );
    let index = inspect(&bytes).unwrap();
    let s = &index.surfaces[SLIDE];
    assert!(s.root_group_fill.is_some());
    assert_eq!(object(&index).fill_reference.as_ref().unwrap().index, 1001);
    assert!(
        s.objects
            .iter()
            .find(|o| o.native_id == 900)
            .unwrap()
            .fill
            .is_some()
    );
    let pic = s
        .objects
        .iter()
        .find(|o| o.kind == SourceObjectKind::Picture)
        .unwrap();
    assert!(matches!(
        pic.picture_fill.as_ref().unwrap().definition,
        SourceFillDefinition::Image(_)
    ));
    let shape_fill = pic
        .fill
        .as_ref()
        .expect("exported picture shape has its own fill");
    let SourceFillDefinition::Solid { color: Some(color) } = &shape_fill.definition else {
        panic!("owned picture frame is white")
    };
    assert!(matches!(
        color.value,
        drawingml::SourceColorValue::Srgb {
            rgb: [255, 255, 255]
        }
    ));
    assert_ne!(
        shape_fill.source_ordinal,
        pic.picture_fill.as_ref().unwrap().source_ordinal
    );
}

#[test]
fn theme_fill_background_and_line_entries_share_the_reader() {
    let index = inspect(&fixture(&format!("{GRADIENT}<a:ln>{GRADIENT}</a:ln>"), "")).unwrap();
    let Some(SourceLineFill::Gradient { gradient, .. }) =
        &object(&index).line.as_ref().unwrap().fill
    else {
        panic!()
    };
    let SourceFillDefinition::Gradient(g) = &object(&index).fill.as_ref().unwrap().definition
    else {
        panic!()
    };
    // Source ordinals differ; lexical declarations do not.
    assert_eq!(g.flip, gradient.flip);
    assert_eq!(
        g.stops.as_ref().unwrap().entries[0].position,
        gradient.stops.as_ref().unwrap().entries[0].position
    );
    for scheme in index
        .themes
        .values()
        .filter_map(|t| t.format_scheme.as_ref())
    {
        assert!(!scheme.fills.is_empty());
        assert!(
            scheme
                .fills
                .iter()
                .chain(&scheme.background_fills)
                .all(|e| e.fill.is_some())
        );
    }
}

#[test]
fn background_properties_and_reference_preserve_absence_and_effect_bindings() {
    let bytes = rewrite(&base(), SLIDE, |xml| {
        xml.replace("<p:spTree>", &format!("<p:bg bwMode=\"gray\"><p:bgPr shadeToTitle=\"0\">{GRADIENT}<a:effectLst><a:blur rad=\"500\"/></a:effectLst></p:bgPr></p:bg><p:spTree>"))
    });
    let index = inspect(&bytes).unwrap();
    let bg = index.surfaces[SLIDE].background.as_ref().unwrap();
    assert_eq!(bg.black_white_mode, Some(NativeBlackWhiteMode::Gray));
    assert!(bg.retained_ordinals.is_empty());
    let SourceBackgroundDefinition::Properties {
        shade_to_title,
        fill,
        effects,
        retained_ordinals,
        ..
    } = &bg.definition
    else {
        panic!()
    };
    let effects::SourceEffectPropertiesDefinition::List { nodes } =
        &effects.as_ref().unwrap().definition
    else {
        panic!()
    };
    assert_eq!(nodes.len(), 1);
    let effects::SourceEffectDefinition::Blur(blur) =
        &index.surfaces[SLIDE].effect_nodes[&nodes[0]].definition
    else {
        panic!()
    };
    assert_eq!(blur.radius.unwrap().get(), 500);
    assert_eq!(*shade_to_title, Some(false));
    assert!(fill.retained_ordinals.is_empty());
    assert_eq!(retained_ordinals, &bg.retained_ordinals);
    for index in [0, 1, 999, 1000, 1001, u32::MAX] {
        let bytes = rewrite(&base(), SLIDE, |xml| {
            xml.replace(
                "<p:spTree>",
                &format!("<p:bg><p:bgRef idx=\"{index}\"/></p:bg><p:spTree>"),
            )
        });
        let result = inspect(&bytes).unwrap();
        let bg = result.surfaces[SLIDE].background.as_ref().unwrap();
        assert!(bg.black_white_mode.is_none());
        let SourceBackgroundDefinition::Reference(r) = &bg.definition else {
            panic!()
        };
        assert_eq!(r.index, index);
        assert!(r.color.is_none());
    }
}

#[test]
fn known_effects_and_opaque_extensions_have_separate_source_bindings() {
    let xml = r#"<a:blipFill future="yes"><a:blip><a:duotone><a:srgbClr val="FFFFFF"/><a:srgbClr val="000000"/></a:duotone><a:extLst><a:ext uri="owned"><a:tile tx="invalid"/><a:gradFill><a:ln cap="bad"/></a:gradFill></a:ext></a:extLst></a:blip><a:tile algn="ctr" future="retained"/></a:blipFill>"#;
    let index = inspect(&fixture(xml, "")).unwrap();
    let f = object(&index).fill.as_ref().unwrap();
    assert_eq!(f.retained_ordinals.len(), 3);
    let SourceFillDefinition::Image(i) = &f.definition else {
        panic!()
    };
    let blip = i.blip.as_ref().unwrap();
    assert_eq!(blip.retained_ordinals.len(), 1);
    assert_eq!(blip.effect_nodes.len(), 1);
    assert!(matches!(
        index.surfaces[SLIDE].effect_nodes[&blip.effect_nodes[0]].definition,
        effects::SourceEffectDefinition::Duotone(_)
    ));
    let Some(SourceImageFillMode::Tile(t)) = &i.mode else {
        panic!()
    };
    assert!(t.translate_x.is_none());
    assert_eq!(t.alignment, Some(NativeFillAlignment::Center));
}

#[test]
fn malformed_fill_grammar_and_ranges_fail_instead_of_becoming_retained_success() {
    for xml in [
        "<a:noFill><a:solidFill/></a:noFill>",
        "<a:noFill/><a:grpFill/>",
        "<a:gradFill><a:gsLst/></a:gradFill>",
        "<a:gradFill><a:gsLst><a:gs pos=\"0\"><a:srgbClr val=\"000000\"/></a:gs></a:gsLst></a:gradFill>",
        "<a:gradFill><a:lin/><a:path/></a:gradFill>",
        "<a:gradFill><a:tileRect/><a:lin/></a:gradFill>",
        "<a:gradFill flip=\"both\"/>",
        "<a:gradFill rotWithShape=\"yes\"/>",
        "<a:gradFill><a:lin ang=\"21600000\"/></a:gradFill>",
        "<a:gradFill><a:lin ang=\"-1\"/></a:gradFill>",
        "<a:gradFill><a:path path=\"ellipse\"/></a:gradFill>",
        "<a:pattFill prst=\"bad\"/>",
        "<a:pattFill><a:fgClr/></a:pattFill>",
        "<a:pattFill><a:bgClr><a:srgbClr val=\"000000\"/></a:bgClr><a:fgClr><a:srgbClr val=\"FFFFFF\"/></a:fgClr></a:pattFill>",
        "<a:blipFill dpi=\"-1\"/>",
        "<a:blipFill dpi=\"4294967296\"/>",
        "<a:blipFill><a:tile/><a:stretch/></a:blipFill>",
        "<a:blipFill><a:blip cstate=\"bad\"/></a:blipFill>",
        "<a:blipFill><a:blip><a:extLst/><a:blur/></a:blip></a:blipFill>",
        "<a:blipFill><a:srcRect/><a:blip/></a:blipFill>",
        "<a:blipFill><a:stretch><a:fillRect/><a:fillRect/></a:stretch></a:blipFill>",
        "<a:solidFill><a:srgbClr val=\"000000\"/><a:schemeClr val=\"accent1\"/></a:solidFill>",
        "<a:solidFill><a:srgbClr val=\"000000\"><a:alpha val=\"0\"><a:t/></a:alpha></a:srgbClr></a:solidFill>",
        "<a:solidFill>unexpected</a:solidFill>",
        "<a:gradFill><a:unknown/></a:gradFill>",
    ] {
        assert!(inspect(&fixture(xml, "")).is_err(), "{xml}");
    }
    for bad in [
        "-1", "100001", "-0.01%", "100.01%", "12.345%", "NaN", "1e4", "",
    ] {
        assert!(
            inspect(&fixture(&GRADIENT.replace("+0050000", bad), "")).is_err(),
            "{bad}"
        );
    }
    for bad in [
        "+1cm",
        ".1mm",
        "1.px",
        "1e4",
        "NaN",
        "27273042316901",
        "-27273042329601",
        "1 px",
    ] {
        assert!(
            inspect(&fixture(
                &format!("<a:blipFill><a:tile tx=\"{bad}\"/></a:blipFill>"),
                ""
            ))
            .is_err(),
            "{bad}"
        );
    }
    for bad in [
        "<p:bg/>",
        "<p:bg><p:bgPr/></p:bg>",
        "<p:bg><p:bgRef idx=\"-1\"/></p:bg>",
        "<p:bg><p:bgRef idx=\"0\"/><p:bgPr><a:noFill/></p:bgPr></p:bg>",
    ] {
        assert!(
            inspect(&rewrite(&base(), SLIDE, |s| s
                .replace("<p:spTree>", &format!("{bad}<p:spTree>"))))
            .is_err(),
            "{bad}"
        );
    }
}

#[test]
fn shared_paint_budget_and_cancellation_cover_surfaces_lines_themes_and_retention() {
    let bytes = fixture(&format!("{GRADIENT}<a:ln>{GRADIENT}</a:ln>"), "");
    for limits in [
        SourceLimits {
            max_paint_elements: 4,
            ..Default::default()
        },
        SourceLimits {
            max_paint_attribute_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            inspect_source(&package(&bytes), limits, &|| false),
            Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
        ));
    }
    assert!(inspect_source(&package(&bytes), SourceLimits::default(), &|| true).is_err());
    let opaque = fixture(
        "<a:blipFill><a:blip><a:extLst><a:ext uri=\"owned\"><a:solidFill future=\"12345678901234567890\"/></a:ext></a:extLst></a:blip></a:blipFill>",
        "",
    );
    assert!(matches!(
        inspect_source(
            &package(&opaque),
            SourceLimits {
                max_paint_attribute_bytes: 10,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
    ));
}

#[test]
fn text_edits_preserve_fill_bytes_and_reindex_bindings() {
    let bytes = fixture(GRADIENT, "");
    let index = inspect(&bytes).unwrap();
    let obj = object(&index);
    let edit = SourceTextEdits {
        expected_source_sha256: index.source_sha256.clone(),
        edits: vec![SourceTextEdit {
            target: SourceTextTarget {
                part: SLIDE.into(),
                object_id: obj.native_id,
                paragraph: 0,
                run: 0,
            },
            expected_text: obj.paragraphs[0][0].text.clone(),
            replacement: "edited & native".into(),
        }],
    };
    let candidate =
        edit_source_text(&package(&bytes), &edit, SourceLimits::default(), &|| false).unwrap();
    assert_eq!(object(&inspect(&candidate).unwrap()).fill, obj.fill);
    let p = package(&candidate);
    let xml = p
        .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
        .unwrap();
    assert!(String::from_utf8(xml).unwrap().contains(GRADIENT));
}
