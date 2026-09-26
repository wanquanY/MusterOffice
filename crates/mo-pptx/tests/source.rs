mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{source::*, *};

fn fixture() -> Vec<u8> {
    let (doc, defaults) = support::input();
    export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn change(bytes: &[u8], part: &str, mutate: impl FnOnce(String) -> String) -> Vec<u8> {
    let source = package(bytes);
    let part = PartName::new(part).unwrap();
    let original =
        String::from_utf8(source.read_part(&part, 1024 * 1024, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, mutate(original).into_bytes())
        .unwrap();
    plan.to_bytes(&source, &|| false).unwrap()
}
fn request(index: &SourceIndex, name: &str, replacement: &str) -> SourceTextEdits {
    let (part, object) = index
        .surfaces
        .iter()
        .find_map(|(part, surface)| {
            surface
                .objects
                .iter()
                .find(|o| o.name == name)
                .map(|o| (part, o))
        })
        .unwrap();
    SourceTextEdits {
        expected_source_sha256: index.source_sha256.clone(),
        edits: vec![SourceTextEdit {
            target: SourceTextTarget {
                part: part.clone(),
                object_id: object.native_id,
                paragraph: 0,
                run: 0,
            },
            expected_text: object.paragraphs[0][0].text.clone(),
            replacement: replacement.into(),
        }],
    }
}

fn explicit_map(slot: &str) -> String {
    format!(
        "<a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"{slot}\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>"
    )
}

#[test]
fn color_map_inheritance_keeps_author_markers_and_candidate_provenance() {
    let bytes = change(&fixture(), "/ppt/slideLayouts/slideLayout2.xml", |s| {
        s.replace("<a:masterClrMapping/>", &explicit_map("accent2"))
    });
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let slide = &index.surfaces["/ppt/slides/slide1.xml"];
    assert!(matches!(
        slide.color_mapping,
        Some(SourceColorMapping::Master { .. })
    ));
    let resolved = slide.resolved_color_mapping.as_ref().unwrap();
    assert_eq!(resolved.part, "/ppt/slideLayouts/slideLayout2.xml");
    let Some(SourceColorMapping::Explicit {
        source_ordinal,
        mapping,
    }) = &index.surfaces[&resolved.part].color_mapping
    else {
        panic!("expected explicit map")
    };
    assert_eq!(*source_ordinal, resolved.source_ordinal);
    assert_eq!(mapping.accent1, theme::ColorSlot::Accent2);
    let edited = edit_source_text(
        &source,
        &request(&index, "title:1", "颜色映射仍来自版式"),
        SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    let after = inspect_source(&package(&edited), SourceLimits::default(), &|| false).unwrap();
    for (part, surface) in &index.surfaces {
        assert_eq!(surface.color_mapping, after.surfaces[part].color_mapping);
        assert_eq!(
            surface.resolved_color_mapping,
            after.surfaces[part].resolved_color_mapping
        );
    }
    let local = change(&bytes, "/ppt/slides/slide1.xml", |s| {
        s.replace("<a:masterClrMapping/>", &explicit_map("accent3"))
    });
    let index = inspect_source(&package(&local), SourceLimits::default(), &|| false).unwrap();
    assert_eq!(
        index.surfaces["/ppt/slides/slide1.xml"]
            .resolved_color_mapping
            .as_ref()
            .unwrap()
            .part,
        "/ppt/slides/slide1.xml"
    );
}

#[test]
fn omitted_color_map_is_not_an_explicit_marker_or_an_invented_default() {
    let bytes = change(&fixture(), "/ppt/slides/slide1.xml", |s| {
        s.replace("<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>", "")
    });
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let slide = &index.surfaces["/ppt/slides/slide1.xml"];
    assert!(slide.color_mapping.is_none());
    assert_eq!(
        slide.resolved_color_mapping.as_ref().unwrap().part,
        "/ppt/slideMasters/slideMaster2.xml"
    );
    let bytes = change(&bytes, "/ppt/slideMasters/slideMaster2.xml", |s| {
        let start = s.find("<p:clrMap ").unwrap();
        let end = start + s[start..].find("/>").unwrap() + 2;
        format!("{}{}", &s[..start], &s[end..])
    });
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    assert!(
        index.surfaces["/ppt/slides/slide1.xml"]
            .resolved_color_mapping
            .is_none()
    );
}

#[test]
fn malformed_color_maps_cannot_produce_partial_success() {
    for replacement in [
        String::new(),
        "<a:masterClrMapping/><a:masterClrMapping/>".into(),
        "<a:masterClrMapping><a:overrideClrMapping/></a:masterClrMapping>".into(),
        explicit_map("bg1"),
        explicit_map("accent2").replace(" bg1=\"lt1\"", ""),
    ] {
        let bytes = change(&fixture(), "/ppt/slides/slide1.xml", |s| {
            s.replace("<a:masterClrMapping/>", &replacement)
        });
        assert!(inspect_source(&package(&bytes), SourceLimits::default(), &|| false).is_err());
    }
}

#[test]
fn native_themes_are_shared_and_selected_without_mutating_author_declarations() {
    use mo_pptx::source::theme::*;
    let bytes = fixture();
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    assert_eq!(index.themes.len(), 2);
    for surface in index.surfaces.values() {
        let selected = surface.theme_selection.colors.as_ref().unwrap();
        let part = &index.themes[&selected.part];
        let colors = part.color_scheme.as_ref().unwrap();
        assert_eq!(selected.source_ordinal, colors.source_ordinal);
        assert_eq!(colors.colors.len(), 12);
        assert!(matches!(
            colors.colors[&ColorSlot::Accent1].value,
            SourceColorValue::Srgb { .. }
        ));
        assert_eq!(
            part.font_scheme
                .as_ref()
                .unwrap()
                .major
                .latin
                .as_ref()
                .unwrap()
                .typeface,
            "Arial"
        );
        assert_eq!(part.format_scheme.as_ref().unwrap().fills.len(), 3);
    }
    let edited = edit_source_text(
        &source,
        &request(&index, "title:1", "主题不能被文本修改压平"),
        SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    let after = inspect_source(&package(&edited), SourceLimits::default(), &|| false).unwrap();
    assert_eq!(index.themes, after.themes);
}

#[test]
fn source_theme_colors_keep_all_models_exact_percentages_and_transform_order() {
    use mo_pptx::source::theme::*;
    let bytes = change(&fixture(), "/ppt/theme/theme2.xml", |s| {
        let start = s.find("<a:accent1>").unwrap();
        let end = start + s[start..].find("</a:accent1>").unwrap() + "</a:accent1>".len();
        let mut s = s;
        s.replace_range(start..end, "<a:accent1><a:scrgbClr r=\"50.123456789%\" g=\"+0001\" b=\"-20000\"><a:alpha val=\"50%\"/><a:lumMod val=\"200000\"/><a:alpha val=\"75000\"/><a:inv/><a:hueOff val=\"-60000\"/></a:scrgbClr></a:accent1>");
        s
    });
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let color = &index.themes["/ppt/theme/theme2.xml"]
        .color_scheme
        .as_ref()
        .unwrap()
        .colors[&ColorSlot::Accent1];
    let SourceColorValue::ScRgb { red, green, blue } = &color.value else {
        panic!("scRGB expected")
    };
    assert_eq!(red.lexical(), "50.123456789%");
    assert_eq!(green.lexical(), "+0001");
    assert_eq!(blue.lexical(), "-20000");
    assert_eq!(color.transforms.len(), 5);
    assert!(matches!(
        color.transforms[0],
        SourceColorTransform::Alpha(_)
    ));
    assert!(matches!(
        color.transforms[2],
        SourceColorTransform::Alpha(_)
    ));
    assert_eq!(color.transforms[3], SourceColorTransform::Inverse);
    assert_eq!(color.transforms[4], SourceColorTransform::HueOff(-60000));
}

#[test]
fn theme_mce_and_retained_declarations_cannot_inject_active_scheme_state() {
    use mo_pptx::source::theme::*;
    let bytes = change(&fixture(), "/ppt/theme/theme2.xml", |s| {
        let s = s.replace("<a:themeElements>", "<q:metadata xmlns:q=\"urn:owned\"><a:clrScheme><a:accent1>ignored foreign payload</a:accent1></a:clrScheme></q:metadata><a:themeElements>");
        let base = s.find("<a:themeElements>").unwrap();
        let start = base + s[base..].find("<a:accent1>").unwrap();
        let end = start + s[start..].find("</a:accent1>").unwrap() + "</a:accent1>".len();
        let selected = s[start..end].to_owned();
        let replacement = format!(
            "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:u=\"urn:unknown\"><mc:Choice Requires=\"u\"><a:accent1><a:srgbClr val=\"bad\"/></a:accent1></mc:Choice><mc:Fallback>{selected}</mc:Fallback></mc:AlternateContent>"
        );
        let mut s = s;
        s.replace_range(start..end, &replacement);
        s
    });
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let theme = &index.themes["/ppt/theme/theme2.xml"];
    assert_eq!(theme.compatibility.selections.len(), 1);
    assert!(theme.compatibility.selections[0].branches[1].selected);
    assert_eq!(theme.color_scheme.as_ref().unwrap().colors.len(), 12);
    assert!(matches!(
        theme.color_scheme.as_ref().unwrap().colors[&ColorSlot::Accent1].value,
        SourceColorValue::Srgb { .. }
    ));
    assert!(theme.notices.iter().any(|n| n.contains("metadata")));
}

#[test]
fn theme_validation_and_aggregate_budgets_prevent_partial_success() {
    let bytes = fixture();
    for mutate in [("<a:majorFont>", "<a:minorFont>"), ("<a:lt1>", "<a:dk1>")] {
        // Keep the XML well-formed while making native declarations duplicate.
        let original_name = mutate.0.trim_start_matches('<').trim_end_matches('>');
        let replacement_name = mutate.1.trim_start_matches('<').trim_end_matches('>');
        let changed = change(&bytes, "/ppt/theme/theme2.xml", |s| {
            s.replace(mutate.0, mutate.1).replace(
                &format!("</{original_name}>"),
                &format!("</{replacement_name}>"),
            )
        });
        assert!(inspect_source(&package(&changed), SourceLimits::default(), &|| false).is_err());
    }
    let source = package(&bytes);
    for limits in [
        SourceLimits {
            max_theme_parts: 1,
            ..SourceLimits::default()
        },
        SourceLimits {
            max_theme_elements: 1,
            ..SourceLimits::default()
        },
        SourceLimits {
            max_theme_attribute_bytes: 1,
            ..SourceLimits::default()
        },
    ] {
        let error = inspect_source(&source, limits, &|| false).unwrap_err();
        assert!(matches!(
            error,
            PptxError::Limit(_) | PptxError::Xml(mo_xml::XmlError::Limit(_))
        ));
    }
}

fn shape_part(mut xml: String, name: &str, ph: &str, transform: &str) -> String {
    let name_pos = xml.find(&format!("name=\"{name}\"")).unwrap();
    let start = xml[..name_pos].rfind("<p:sp>").unwrap();
    let end = name_pos + xml[name_pos..].find("</p:sp>").unwrap() + "</p:sp>".len();
    let mut shape = xml[start..end].replace("<p:nvPr/>", &format!("<p:nvPr><p:ph {ph}/></p:nvPr>"));
    let a = shape.find("<a:xfrm").unwrap();
    let b = a + shape[a..].find("</a:xfrm>").unwrap() + "</a:xfrm>".len();
    shape.replace_range(a..b, transform);
    xml.replace_range(start..end, &shape);
    xml
}

fn placeholder_fixture() -> Vec<u8> {
    let bytes = change(&fixture(), "/ppt/slideMasters/slideMaster2.xml", |s| {
        shape_part(
            s,
            "footer:master",
            "type=\"body\" idx=\"91\"",
            "<a:xfrm><a:off x=\"111\" y=\"222\"/><a:ext cx=\"333\" cy=\"444\"/></a:xfrm>",
        )
    });
    let bytes = change(&bytes, "/ppt/slideLayouts/slideLayout2.xml", |s| {
        shape_part(
            s,
            "rule:layout",
            "idx=\"7\"",
            "<a:xfrm><a:ext cx=\"555\" cy=\"666\"/></a:xfrm>",
        )
    });
    change(&bytes, "/ppt/slides/slide1.xml", |s| {
        shape_part(s, "title:1", "type=\"body\" idx=\"7\"", "")
    })
}

#[test]
fn placeholder_dimensions_resolve_per_property_without_flattening_author_data() {
    let bytes = placeholder_fixture();
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let slide = &index.surfaces["/ppt/slides/slide1.xml"];
    assert_eq!(
        slide.links.layout.as_deref(),
        Some("/ppt/slideLayouts/slideLayout2.xml")
    );
    assert_eq!(
        slide.effective_theme.base.as_ref().unwrap().part,
        "/ppt/theme/theme2.xml"
    );
    let object = slide.objects.iter().find(|o| o.name == "title:1").unwrap();
    assert_eq!(object.transform, None);
    let origin = object.resolution.origin.as_ref().unwrap();
    assert_eq!(origin.value.x.get(), 111);
    assert_eq!(origin.value.y.get(), 222);
    assert_eq!(
        origin.declared_by.part,
        "/ppt/slideMasters/slideMaster2.xml"
    );
    let size = object.resolution.size.as_ref().unwrap();
    assert_eq!(size.value.width.get(), 555);
    assert_eq!(size.declared_by.part, "/ppt/slideLayouts/slideLayout2.xml");
    assert!(matches!(
        object.resolution.placeholder_match,
        SourcePlaceholderMatch::Matched {
            rule: PlaceholderMatchRule::SlideIndex,
            ..
        }
    ));
    let layout = &index.surfaces["/ppt/slideLayouts/slideLayout2.xml"].objects[0];
    assert_eq!(layout.placeholder.as_ref().unwrap().kind, None);
    assert_eq!(
        layout.placeholder.as_ref().unwrap().effective_kind(),
        PlaceholderKind::Object
    );
    assert!(matches!(
        layout.resolution.placeholder_match,
        SourcePlaceholderMatch::Matched {
            rule: PlaceholderMatchRule::MasterType,
            ..
        }
    ));
    let output = edit_source_text(
        &source,
        &request(&index, "title:1", "源继承仍保留"),
        SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    let after = inspect_source(&package(&output), SourceLimits::default(), &|| false).unwrap();
    let after = after.surfaces["/ppt/slides/slide1.xml"]
        .objects
        .iter()
        .find(|o| o.name == "title:1")
        .unwrap();
    assert_eq!(after.resolution, object.resolution);
    assert_eq!(after.transform, None);
}

#[test]
fn explicit_zero_dimensions_and_missing_or_detached_matches_do_not_invent_geometry() {
    for (attributes, expected) in [
        ("idx=\"700\"", SourcePlaceholderMatch::Unmatched),
        ("idx=\"4294967295\"", SourcePlaceholderMatch::Detached),
        (
            "type=\"hdr\" idx=\"7\"",
            SourcePlaceholderMatch::UnsupportedContext,
        ),
    ] {
        let bytes = change(&placeholder_fixture(), "/ppt/slides/slide1.xml", |s| {
            s.replace("type=\"body\" idx=\"7\"", attributes)
        });
        let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
        let object = index.surfaces["/ppt/slides/slide1.xml"]
            .objects
            .iter()
            .find(|o| o.name == "title:1")
            .unwrap();
        assert_eq!(object.resolution.placeholder_match, expected);
        assert_eq!(object.resolution.origin, None);
        assert_eq!(object.resolution.size, None);
    }
    let bytes = change(&placeholder_fixture(), "/ppt/slides/slide1.xml", |s| {
        s.replacen("</p:nvSpPr><p:spPr>", "</p:nvSpPr><p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/></a:xfrm>", 1)
    });
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let object = index.surfaces["/ppt/slides/slide1.xml"]
        .objects
        .iter()
        .find(|o| o.name == "title:1")
        .unwrap();
    assert_eq!(object.resolution.origin.as_ref().unwrap().value.x.get(), 0);
    assert_eq!(
        object.resolution.size.as_ref().unwrap().value.width.get(),
        0
    );
    assert_eq!(
        object.resolution.origin.as_ref().unwrap().declared_by.part,
        "/ppt/slides/slide1.xml"
    );
}

#[test]
fn ambiguous_placeholders_are_reported_without_first_match_geometry() {
    let bytes = change(
        &placeholder_fixture(),
        "/ppt/slideLayouts/slideLayout2.xml",
        |s| {
            let start = s.find("<p:sp>").unwrap();
            let end = start + s[start..].find("</p:sp>").unwrap() + 7;
            let duplicate = s[start..end]
                .replace("id=\"11\"", "id=\"888\"")
                .replace("rule:layout", "duplicate:layout");
            s.replace("</p:spTree>", &format!("{duplicate}</p:spTree>"))
        },
    );
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let object = index.surfaces["/ppt/slides/slide1.xml"]
        .objects
        .iter()
        .find(|o| o.name == "title:1")
        .unwrap();
    assert_eq!(
        object.resolution.placeholder_match,
        SourcePlaceholderMatch::Ambiguous {
            part: "/ppt/slideLayouts/slideLayout2.xml".into(),
            candidates: 2
        }
    );
    assert_eq!(object.resolution.origin, None);
    assert_eq!(object.resolution.size, None);
}

#[test]
fn invalid_placeholder_attributes_are_rejected() {
    for bad in [
        "type=\"madeUp\"",
        "idx=\"4294967296\"",
        "orient=\"diagonal\"",
        "sz=\"huge\"",
        "hasCustomPrompt=\"yes\"",
    ] {
        let bytes = change(&placeholder_fixture(), "/ppt/slides/slide1.xml", |s| {
            s.replace("type=\"body\" idx=\"7\"", bad)
        });
        assert!(
            inspect_source(&package(&bytes), SourceLimits::default(), &|| false).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn unsupported_parent_placeholder_context_does_not_leak_geometry_into_descendants() {
    let bytes = change(
        &placeholder_fixture(),
        "/ppt/slideMasters/slideMaster2.xml",
        |s| s.replace("idx=\"91\"", "idx=\"4294967295\""),
    );
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let layout = &index.surfaces["/ppt/slideLayouts/slideLayout2.xml"].objects[0];
    assert_eq!(
        layout.resolution.placeholder_match,
        SourcePlaceholderMatch::UnsupportedContext
    );
    assert_eq!(layout.resolution.origin, None);
    assert_eq!(
        layout.resolution.size.as_ref().unwrap().value.width.get(),
        555
    );
    let slide = index.surfaces["/ppt/slides/slide1.xml"]
        .objects
        .iter()
        .find(|o| o.name == "title:1")
        .unwrap();
    assert_eq!(
        slide.resolution.placeholder_match,
        SourcePlaceholderMatch::UnsupportedContext
    );
    assert_eq!(slide.resolution.size, None);
}

#[test]
fn source_text_edit_preserves_signed_and_multi_turn_transform_tokens() {
    for angle in [-8_100_000, 24_300_000, i32::MIN, i32::MAX] {
        let bytes = change(&fixture(), "/ppt/slides/slide2.xml", |xml| {
            assert_eq!(xml.matches("rot=\"21300000\"").count(), 1);
            xml.replace("rot=\"21300000\"", &format!("rot=\"{angle}\""))
        });
        let source = package(&bytes);
        let before = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
        let change = request(&before, "child:2a", "Edited inside the same rotated group");
        let edited =
            edit_source_text(&source, &change, SourceLimits::default(), &|| false).unwrap();
        let candidate = package(&edited);
        let after = inspect_source(&candidate, SourceLimits::default(), &|| false).unwrap();
        for index in [&before, &after] {
            let group = index.surfaces["/ppt/slides/slide2.xml"]
                .objects
                .iter()
                .find(|o| o.name == "group:2")
                .unwrap();
            assert_eq!(group.transform.as_ref().unwrap().rotation, Some(angle));
        }
        let part = PartName::new("/ppt/slides/slide2.xml").unwrap();
        let before_xml =
            String::from_utf8(source.read_part(&part, 1024 * 1024, &|| false).unwrap()).unwrap();
        let after_xml =
            String::from_utf8(candidate.read_part(&part, 1024 * 1024, &|| false).unwrap()).unwrap();
        assert_eq!(
            after_xml,
            before_xml.replace(&change.edits[0].expected_text, &change.edits[0].replacement)
        );
    }
}

#[test]
fn real_pptx_projection_resolves_pages_surfaces_native_identity_and_groups() {
    let bytes = fixture();
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    assert_eq!(index.slides.len(), 2);
    assert_eq!(index.surfaces.len(), 6);
    assert_eq!(
        index
            .surfaces
            .values()
            .map(|s| s.objects.len())
            .sum::<usize>(),
        15
    );
    assert_eq!(index.slides[0].native_id, 256);
    let slide = &index.surfaces[&index.slides[1].part];
    let group = slide.objects.iter().find(|o| o.name == "group:2").unwrap();
    let child = slide.objects.iter().find(|o| o.name == "child:2a").unwrap();
    assert_eq!(child.parent_group, Some(group.native_id));
    assert_eq!(group.transform.as_ref().unwrap().rotation, Some(21300000));
    assert!(
        index
            .surfaces
            .values()
            .flat_map(|s| &s.objects)
            .flat_map(|o| &o.paragraphs)
            .flatten()
            .any(|r| r.text.contains("中文 / العربية / 🚀 / é"))
    );
    assert!(index.notices.iter().any(|n| n.contains("partial")));
}

#[test]
fn text_edit_reopens_candidate_and_preserves_unknown_xml_and_other_parts() {
    let bytes = change(&fixture(), "/ppt/slides/slide1.xml", |s| {
        s.replace("<p:cSld", "<!-- untouched comment --><p:cSld xmlns:q=\"urn:owned-fixture\" q:metadata=\"keep\"")
        .replace("</p:sld>", "<p:extLst><p:ext uri=\"urn:owned-extension\"><q:payload xmlns:q=\"urn:owned-fixture\">keep &amp; exact</q:payload></p:ext></p:extLst></p:sld>")
    });
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let edits = request(&index, "unicode:1", "修改 😀 é & < >\r\n新段内文本");
    let output = edit_source_text(&source, &edits, SourceLimits::default(), &|| false).unwrap();
    let changed = package(&output);
    let part = PartName::new(&edits.edits[0].target.part).unwrap();
    for (name, info) in source.parts() {
        if name != &part {
            assert_eq!(info.sha256, changed.parts()[name].sha256, "{name}");
        }
    }
    assert_eq!(source.relationships(), changed.relationships());
    let original =
        String::from_utf8(source.read_part(&part, 1024 * 1024, &|| false).unwrap()).unwrap();
    let actual =
        String::from_utf8(changed.read_part(&part, 1024 * 1024, &|| false).unwrap()).unwrap();
    let expected = original.replace(
        &edits.edits[0].expected_text,
        "修改 😀 é &amp; &lt; &gt;&#xD;\n新段内文本",
    );
    assert_eq!(actual, expected);
    assert!(matches!(
        edit_source_text(&changed, &edits, SourceLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}

#[test]
fn noop_copy_and_multi_part_edits_are_atomic_with_source_preconditions() {
    let bytes = fixture();
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let mut edits = request(&index, "title:1", "first");
    edits.edits[0].replacement = edits.edits[0].expected_text.clone();
    assert_eq!(
        edit_source_text(&source, &edits, SourceLimits::default(), &|| false).unwrap(),
        bytes
    );
    edits.edits[0].replacement = "first".into();
    edits
        .edits
        .extend(request(&index, "title:2", "second").edits);
    let output = edit_source_text(&source, &edits, SourceLimits::default(), &|| false).unwrap();
    let result = inspect_source(&package(&output), SourceLimits::default(), &|| false).unwrap();
    assert_eq!(
        request(&result, "title:1", "").edits[0].expected_text,
        "first"
    );
    assert_eq!(
        request(&result, "title:2", "").edits[0].expected_text,
        "second"
    );
    edits.edits[1].expected_text = "stale".into();
    assert!(matches!(
        edit_source_text(&source, &edits, SourceLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
    assert_eq!(
        inspect_source(&source, SourceLimits::default(), &|| false).unwrap(),
        index
    );
    edits.edits.pop();
    edits.edits.push(edits.edits[0].clone());
    assert!(matches!(
        edit_source_text(&source, &edits, SourceLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}

#[test]
fn malformed_identity_and_relationships_never_produce_a_partial_success() {
    for (part, from, to) in [
        ("/ppt/presentation.xml", "id=\"257\"", "id=\"256\""),
        ("/ppt/presentation.xml", "r:id=\"rId3\"", "r:id=\"absent\""),
        (
            "/ppt/slides/slide1.xml",
            "name=\"title:1\"",
            "name=\"title:1\" id=\"999\"",
        ),
    ] {
        // Duplicate XML attributes fail in OPC before the format projection.
        let bytes = fixture();
        let source = package(&bytes);
        let part = PartName::new(part).unwrap();
        let original =
            String::from_utf8(source.read_part(&part, 1024 * 1024, &|| false).unwrap()).unwrap();
        assert!(original.contains(from));
        let mut plan = RewritePlan::new();
        plan.replace_part(part, original.replace(from, to).into_bytes())
            .unwrap();
        if let Ok(changed) = plan.to_bytes(&source, &|| false) {
            assert!(
                inspect_source(&package(&changed), SourceLimits::default(), &|| false).is_err()
            );
        }
    }
}

#[test]
fn timing_references_in_active_and_inactive_branches_block_text_mutation() {
    for extra in [
        "<p:timing/>",
        "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:u=\"urn:future\"><mc:Choice Requires=\"u\"><p:timing/></mc:Choice><mc:Fallback/></mc:AlternateContent>",
    ] {
        let bytes = change(&fixture(), "/ppt/slides/slide1.xml", |s| {
            s.replace("</p:sld>", &format!("{extra}</p:sld>"))
        });
        let source = package(&bytes);
        let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
        let edits = request(&index, "title:1", "new");
        assert!(matches!(
            edit_source_text(&source, &edits, SourceLimits::default(), &|| false),
            Err(PptxError::Unsupported(_))
        ));
        assert!(
            !index.surfaces[&edits.edits[0].target.part]
                .objects
                .iter()
                .flat_map(|o| &o.paragraphs)
                .flatten()
                .any(|r| r.editable)
        );
    }
}

#[test]
fn source_bindings_use_physical_ordinals_after_mce_projection_and_preserve_branches() {
    let mut wrapper = String::new();
    let bytes = change(&fixture(), "/ppt/slides/slide1.xml", |s| {
        let start = s.find("<p:sp>").unwrap();
        let end = start + s[start..].find("</p:sp>").unwrap() + "</p:sp>".len();
        let shape = &s[start..end];
        wrapper = format!(
            "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:u=\"urn:future\"><mc:Choice Requires=\"u\">{shape}</mc:Choice><mc:Fallback>{shape}</mc:Fallback></mc:AlternateContent>"
        );
        format!("{}{}{}", &s[..start], wrapper, &s[end..])
    });
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let surface = &index.surfaces["/ppt/slides/slide1.xml"];
    assert_eq!(surface.objects.len(), 6);
    assert!(surface.compatibility.selections[0].branches[1].selected);
    assert!(surface.text_edit_barriers.is_empty());
    assert_eq!(
        surface
            .objects
            .iter()
            .find(|o| o.name == "title:1")
            .unwrap()
            .paragraphs[0][0]
            .edit_constraint,
        Some(SourceTextConstraint::CompatibilityBranch)
    );
    let edit = request(&index, "title:1", "new");
    assert!(matches!(
        edit_source_text(&source, &edit, SourceLimits::default(), &|| false),
        Err(PptxError::Unsupported(_))
    ));
    let edit = request(&index, "unicode:1", "outside selected alternatives 😀");
    let output = edit_source_text(&source, &edit, SourceLimits::default(), &|| false).unwrap();
    let part = PartName::new("/ppt/slides/slide1.xml").unwrap();
    let actual = String::from_utf8(
        package(&output)
            .read_part(&part, 1024 * 1024, &|| false)
            .unwrap(),
    )
    .unwrap();
    assert!(actual.contains(&wrapper));
    let after = inspect_source(&package(&output), SourceLimits::default(), &|| false).unwrap();
    assert_eq!(
        request(&after, "unicode:1", "").edits[0].expected_text,
        edit.edits[0].replacement
    );
}

#[test]
fn compatibility_wrappers_do_not_hide_pages_or_discard_source_leaf_structure() {
    let bytes = change(&fixture(), "/ppt/presentation.xml", |s| {
        let body = &s[s.find("?>").unwrap() + 2..];
        format!(
            "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><mc:Choice Requires=\"p\">{body}</mc:Choice><mc:Fallback><ignored/></mc:Fallback></mc:AlternateContent>"
        )
    });
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    assert_eq!(index.slides.len(), 2);
    assert!(index.main_compatibility.selections[0].branches[0].selected);
    let bytes = change(&fixture(), "/ppt/slides/slide1.xml", |s| {
        s.replacen("<a:t>","<a:t xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:q=\"urn:ignored\" mc:Ignorable=\"q\"><q:metadata/>",1)
    });
    let source = package(&bytes);
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let edit = request(&index, "title:1", "new");
    assert_eq!(
        index.surfaces[&edit.edits[0].target.part]
            .objects
            .iter()
            .find(|o| o.name == "title:1")
            .unwrap()
            .paragraphs[0][0]
            .edit_constraint,
        Some(SourceTextConstraint::StructuredLeaf)
    );
    assert!(matches!(
        edit_source_text(&source, &edit, SourceLimits::default(), &|| false),
        Err(PptxError::Unsupported(_))
    ));
}

#[test]
fn reader_and_editor_enforce_cancellation_and_projection_budgets() {
    let bytes = fixture();
    let source = package(&bytes);
    assert!(matches!(
        inspect_source(&source, SourceLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    for limits in [
        SourceLimits {
            max_objects: 1,
            ..Default::default()
        },
        SourceLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
        SourceLimits {
            max_surfaces: 1,
            ..Default::default()
        },
    ] {
        assert!(inspect_source(&source, limits, &|| false).is_err());
    }
    let index = inspect_source(&source, SourceLimits::default(), &|| false).unwrap();
    let edits = request(&index, "title:1", "new");
    assert!(matches!(
        edit_source_text(
            &source,
            &edits,
            SourceLimits {
                max_edits: 0,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
}
