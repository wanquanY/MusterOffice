#[path = "table_styles/support.rs"]
mod support;
use mo_opc::{PartName, Relationship};
use mo_pptx::source::{table::styles::*, text::NativeFontCollectionIndex, *};
use support::{table, *};

fn inline_style(index: &SourceIndex) -> &SourceTableStyle {
    index.surfaces[table::SLIDE].objects[0]
        .table
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap()
        .inline_style
        .as_deref()
        .unwrap()
}
fn without_ordinals(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(v) => {
            v.remove("sourceOrdinal");
            // Effect-node keys are physical ordinals; compare their values in order.
            if let Some(serde_json::Value::Object(nodes)) = v.get_mut("effectNodes") {
                let values = nodes.values().cloned().collect::<Vec<_>>();
                *v.get_mut("effectNodes").unwrap() = serde_json::to_value(values).unwrap();
            }
            for child in v.values_mut() {
                without_ordinals(child);
            }
        }
        serde_json::Value::Array(a) => {
            for v in a {
                without_ordinals(v);
            }
        }
        _ => (),
    }
}
#[test]
fn inline_and_part_styles_read_all_regions_fonts_paint_and_borders() {
    let b = table::bytes();
    let external = catalog(&b, &list(&full_style("tblStyle", ID)));
    evidence("catalog-without-reference", &external);
    evidence(
        "catalog-referenced",
        &inline(&external, &format!("<a:tableStyleId>{ID}</a:tableStyleId>")),
    );
    let i = read(&external);
    let catalog = i.table_styles.as_ref().unwrap();
    assert_eq!(catalog.part, STYLES);
    assert_eq!(catalog.default_style_id, ID);
    assert_eq!(
        catalog.sha256,
        package(&external).parts()[&PartName::new(STYLES).unwrap()].sha256
    );
    let s = &catalog.styles[ID];
    assert_eq!(s.parts.len(), 13);
    assert!(s.retained_ordinals.is_empty());
    assert_eq!(s.effect_nodes.len(), 1);
    assert!(matches!(
        s.background.as_ref().unwrap().fill,
        Some(SourceTableStyleFill::Reference { .. })
    ));
    assert!(matches!(
        s.background.as_ref().unwrap().effects,
        Some(SourceTableStyleEffects::Direct { .. })
    ));
    for (region, part) in &s.parts {
        assert!(part.retained_ordinals.is_empty());
        let tx = part.text.as_ref().unwrap();
        assert_eq!(tx.bold, Some(TableOnOff::On));
        assert_eq!(tx.italic, Some(TableOnOff::Off));
        assert_eq!(tx.color.as_ref().unwrap().transforms.len(), 1);
        if *region == TableStyleRegion::WholeTbl {
            let Some(SourceTableFontStyle::Collection { fonts, .. }) = &tx.font else {
                panic!()
            };
            assert_eq!(fonts.latin.as_ref().unwrap().typeface, "Owned Latin");
            assert_eq!(fonts.east_asian.as_ref().unwrap().typeface, "Owned East");
            assert_eq!(
                fonts.complex_script.as_ref().unwrap().typeface,
                "Owned Complex"
            );
            assert_eq!(fonts.supplemental.len(), 2);
            assert_eq!(fonts.supplemental[1].typeface, "Second declaration");
        } else {
            assert!(matches!(
                tx.font,
                Some(SourceTableFontStyle::Reference {
                    index: NativeFontCollectionIndex::Minor,
                    color: None,
                    ..
                })
            ));
        }
        let cell = part.cell.as_ref().unwrap();
        assert!(matches!(
            cell.fill,
            Some(SourceTableStyleFill::Direct { .. })
        ));
        let borders = cell.borders.as_ref().unwrap();
        for (edge, value) in borders.edges.iter().enumerate() {
            assert_eq!(
                matches!(value, Some(SourceTableStyleLine::Direct { .. })),
                edge % 2 == 0
            );
            assert_eq!(
                matches!(value, Some(SourceTableStyleLine::Reference { .. })),
                edge % 2 == 1
            );
        }
    }
    let inline_bytes = inline(&b, &full_style("tableStyle", ID));
    evidence("inline-all-regions", &inline_bytes);
    let i2 = read(&inline_bytes);
    let local = inline_style(&i2);
    // Root and all declaration ordinals refer to different actual XML parts.
    assert_ne!(s.source_ordinal, local.source_ordinal);
    let mut a = serde_json::to_value(s).unwrap();
    let mut b = serde_json::to_value(local).unwrap();
    // Effect lists own ordinal references as well as catalog keys.
    a["background"]["effects"] = serde_json::Value::Null;
    b["background"]["effects"] = serde_json::Value::Null;
    without_ordinals(&mut a);
    without_ordinals(&mut b);
    assert_eq!(a, b);
    let json = serde_json::to_vec(&i2).unwrap();
    assert_eq!(serde_json::from_slice::<SourceIndex>(&json).unwrap(), i2);
}
#[test]
fn absent_explicit_default_and_optional_font_color_remain_distinct() {
    let body = "<a:wholeTbl><a:tcTxStyle/></a:wholeTbl><a:firstRow><a:tcTxStyle b=\"def\" i=\"off\"><a:fontRef idx=\"none\"/></a:tcTxStyle></a:firstRow>";
    let bytes = inline(&table::bytes(), &style("tableStyle", ID, body));
    evidence("inline-defaults-font-reference", &bytes);
    let i = read(&bytes);
    let s = inline_style(&i);
    assert_eq!(
        s.parts[&TableStyleRegion::WholeTbl]
            .text
            .as_ref()
            .unwrap()
            .bold,
        None
    );
    let tx = s.parts[&TableStyleRegion::FirstRow].text.as_ref().unwrap();
    assert_eq!(tx.bold, Some(TableOnOff::Def));
    assert_eq!(tx.italic, Some(TableOnOff::Off));
    assert!(matches!(
        tx.font,
        Some(SourceTableFontStyle::Reference {
            index: NativeFontCollectionIndex::None,
            color: None,
            ..
        })
    ));
    let base = read(&table::bytes());
    assert!(base.table_styles.is_none());
    assert!(
        serde_json::to_value(base)
            .unwrap()
            .get("tableStyles")
            .is_none()
    );
}
#[test]
fn retained_style_content_is_scoped_to_its_region_or_background() {
    let body = "<a:tblBg unknown=\"keep\"><a:fill><a:noFill/></a:fill></a:tblBg><a:wholeTbl><a:tcTxStyle b=\"on\"/></a:wholeTbl><a:firstRow><a:tcTxStyle><a:srgbClr val=\"123456\"><a:futureTransform/></a:srgbClr></a:tcTxStyle><a:tcStyle><a:cell3D><a:bevel/></a:cell3D></a:tcStyle></a:firstRow><a:extLst><a:ext uri=\"owned\"><a:wholeTbl/></a:ext></a:extLst>";
    let i = read(&inline(&table::bytes(), &style("tableStyle", ID, body)));
    let s = inline_style(&i);
    assert_eq!(s.retained_ordinals.len(), 1);
    assert_eq!(s.background.as_ref().unwrap().retained_ordinals.len(), 1);
    assert!(
        s.parts[&TableStyleRegion::WholeTbl]
            .retained_ordinals
            .is_empty()
    );
    let first = &s.parts[&TableStyleRegion::FirstRow];
    assert_eq!(first.retained_ordinals.len(), 2);
    assert_eq!(
        first.retained_ordinals[1],
        first.cell.as_ref().unwrap().cell_3d_ordinal.unwrap()
    );
    assert_eq!(s.parts.len(), 2);
}
#[test]
fn style_part_uses_shared_mce_projection_with_physical_ordinals() {
    let chosen = style("tblStyle", ID, "<a:wholeTbl/>");
    let body = format!(
        "<mc:AlternateContent><mc:Choice Requires=\"future\">{} </mc:Choice><mc:Fallback>{chosen}</mc:Fallback></mc:AlternateContent>",
        style("tblStyle", ID2, "")
    );
    let xml = list(&body).replace(" def=", " xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:future=\"urn:owned-future\" def=");
    let i = read(&catalog(&table::bytes(), &xml));
    let s = i.table_styles.unwrap();
    assert_eq!(s.styles.len(), 1);
    assert!(s.styles.contains_key(ID));
    assert!(s.styles[ID].source_ordinal > 2);
    assert_eq!(s.compatibility.selections.len(), 1);
    let branches = &s.compatibility.selections[0].branches;
    assert!(!branches[0].selected);
    assert!(branches[1].fallback && branches[1].selected);
}
#[test]
fn malformed_known_style_grammar_never_becomes_a_resolved_declaration() {
    let invalid = [
        "<a:wholeTbl/><a:wholeTbl/>",
        "<a:firstRow/><a:wholeTbl/>",
        "<a:wholeTbl><a:tcStyle/><a:tcTxStyle/></a:wholeTbl>",
        "<a:wholeTbl><a:tcTxStyle b=\"true\"/></a:wholeTbl>",
        "<a:wholeTbl><a:tcTxStyle><a:fontRef idx=\"bad\"/></a:tcTxStyle></a:wholeTbl>",
        "<a:wholeTbl><a:tcTxStyle><a:font><a:latin typeface=\"Only Latin\"/></a:font></a:tcTxStyle></a:wholeTbl>",
        "<a:wholeTbl><a:tcTxStyle><a:fontRef idx=\"minor\"/><a:fontRef idx=\"major\"/></a:tcTxStyle></a:wholeTbl>",
        "<a:wholeTbl><a:tcStyle><a:tcBdr><a:left/></a:tcBdr></a:tcStyle></a:wholeTbl>",
        "<a:wholeTbl><a:tcStyle><a:fill/></a:tcStyle></a:wholeTbl>",
        "<a:wholeTbl><a:tcStyle><a:fill><a:noFill/><a:grpFill/></a:fill></a:tcStyle></a:wholeTbl>",
        "<a:tblBg><a:effect/></a:tblBg>",
        "<a:wholeTbl><a:tcTxStyle>unexpected</a:tcTxStyle></a:wholeTbl>",
        "<a:wholeTbl><a:tcTxStyle><a:srgbClr val=\"abcdef\"><a:alpha val=\"100001\"/></a:srgbClr></a:tcTxStyle></a:wholeTbl>",
    ];
    let base = table::bytes();
    for body in invalid {
        for external in [true, false] {
            let b = if external {
                catalog(&base, &list(&style("tblStyle", ID, body)))
            } else {
                inline(&base, &style("tableStyle", ID, body))
            };
            assert!(
                inspect_source(&package(&b), Default::default(), &|| false).is_err(),
                "accepted {body}, external={external}"
            );
        }
    }
}
#[test]
fn style_part_binding_and_identity_are_validated() {
    let base = table::bytes();
    for xml in [
        list(&style("tblStyle", "invalid-guid", "")),
        list(&(style("tblStyle", ID, "") + &style("tblStyle", &ID.to_lowercase(), ""))),
        list("").replace(&format!(" def=\"{ID}\""), ""),
        list("").replace("tblStyleLst", "notAStyleList"),
    ] {
        let b = catalog(&base, &xml);
        assert!(inspect_source(&package(&b), Default::default(), &|| false).is_err());
    }
    let xml = list(&style("tblStyle", ID, ""));
    for b in [
        catalog_with(&base, &xml, "application/xml", |_, _| ()),
        catalog_with(&base, &xml, TYPE, |source, rels| {
            rels.push(
                Relationship::new(
                    source,
                    "rSecondStyles".into(),
                    format!("{R}/tableStyles"),
                    "tableStyles.xml".into(),
                    false,
                )
                .unwrap(),
            );
        }),
        catalog_with(&base, &xml, TYPE, |source, rels| {
            *rels.last_mut().unwrap() = Relationship::new(
                source,
                "rTableStyles".into(),
                format!("{R}/tableStyles"),
                "https://example.invalid/styles.xml".into(),
                true,
            )
            .unwrap();
        }),
    ] {
        assert!(inspect_source(&package(&b), Default::default(), &|| false).is_err());
    }
    // Default insertion identity need not be present (built-in catalogs exist).
    assert!(
        read(&catalog(&base, &list("")))
            .table_styles
            .unwrap()
            .styles
            .is_empty()
    );
}
#[test]
fn budgets_are_aggregate_across_inline_styles_and_the_shared_part() {
    let base = table::bytes();
    let b = catalog(
        &inline(&base, &style("tableStyle", ID, "")),
        &list(&style("tblStyle", ID2, "")),
    );
    let p = package(&b);
    assert!(
        inspect_source(
            &p,
            SourceLimits {
                max_table_styles: 2,
                ..Default::default()
            },
            &|| false
        )
        .is_ok()
    );
    assert!(
        inspect_source(
            &p,
            SourceLimits {
                max_table_styles: 1,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    for (elements, attributes) in [(0, usize::MAX), (usize::MAX, 0)] {
        assert!(
            inspect_source(
                &p,
                SourceLimits {
                    max_table_elements: elements,
                    max_table_attribute_bytes: attributes,
                    ..Default::default()
                },
                &|| false
            )
            .is_err()
        );
    }
    assert!(inspect_source(&p, Default::default(), &|| true).is_err());
}
#[test]
fn editing_table_text_preserves_inline_styles_catalog_and_every_other_part() {
    let b = catalog(
        &inline(&table::bytes(), &full_style("tableStyle", ID)),
        &list(&full_style("tblStyle", ID2)),
    );
    let p = package(&b);
    let i = read(&b);
    let o = &i.surfaces[table::SLIDE].objects[0];
    let old = &o.paragraphs[0][0].text;
    let q = SourceTextEdits {
        expected_source_sha256: i.source_sha256.clone(),
        edits: vec![SourceTextEdit {
            target: SourceTextTarget {
                part: table::SLIDE.into(),
                object_id: o.native_id,
                paragraph: 0,
                run: 0,
            },
            expected_text: old.clone(),
            replacement: "edited table content".into(),
        }],
    };
    let b2 = edit_source_text(&p, &q, Default::default(), &|| false).unwrap();
    let p2 = package(&b2);
    let i2 = read(&b2);
    assert_eq!(i.table_styles, i2.table_styles);
    assert_eq!(inline_style(&i), inline_style(&i2));
    assert_eq!(
        p.parts().keys().collect::<Vec<_>>(),
        p2.parts().keys().collect::<Vec<_>>()
    );
    for (part, before) in p.parts() {
        if part.as_str() != table::SLIDE {
            assert_eq!(before.sha256, p2.parts()[part].sha256, "{part}");
        } else {
            let xml = String::from_utf8(p.read_part(part, 1 << 24, &|| false).unwrap()).unwrap();
            let xml2 = String::from_utf8(p2.read_part(part, 1 << 24, &|| false).unwrap()).unwrap();
            let escaped = old
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            assert_eq!(xml.replacen(&escaped, "edited table content", 1), xml2);
        }
    }
}
