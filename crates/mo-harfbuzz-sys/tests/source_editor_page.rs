//! Real raster and font execution; page queries retain the exact rendered frame.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_group_images.rs"]
mod support;
#[allow(dead_code)]
#[path = "../../../tools/test-support/table_text_fixture.rs"]
mod tables;
use mo_common::Emu;
use mo_geometry::{Fixed, Point};
use mo_harfbuzz_sys::NativeShaper;
use mo_opc::Package;
use mo_pptx::source::images::PackageImages;
use mo_presentation_compile::{
    source_editor_page::{self, *},
    source_frame::interaction::{FrameInteractionLimits, FrameTextPosition},
    source_page::SourcePageError,
    source_resource_page::{self, TextPageContext},
};
use mo_skia_sys::NativeRaster;
use mo_text::{
    interaction::{Affinity, CaretMove, TextPosition},
    manifest::{FontManifest, PreparedManifest},
};
use support::*;
const FONT_BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/owned-interaction.ttf");
fn fonts() -> FontManifest {
    let mut m: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/tracking-manifest.json"
    ))
    .unwrap();
    m["fonts"][0]["expectedSha256"] =
        serde_json::json!("74c1ab9163bae9fdd7beb62dbe4e02a646a55f609bd48071bdc0f651e7180db8");
    m["fonts"][0]["byteLength"] = serde_json::json!(FONT_BYTES.len().to_string());
    serde_json::from_value(m).unwrap()
}
fn f(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
fn position(offset: u32) -> FrameTextPosition {
    FrameTextPosition {
        paragraph: 0,
        position: TextPosition {
            scalar_offset: offset,
            affinity: Affinity::Downstream,
        },
    }
}
fn caret(frame: u32, offset: u32) -> PageTextQuery {
    PageTextQuery {
        frame,
        action: PageTextAction::Caret {
            position: position(offset),
        },
    }
}
fn select(frame: u32, a: u32, b: u32) -> PageTextQuery {
    PageTextQuery {
        frame,
        action: PageTextAction::Selection {
            anchor: position(a),
            focus: position(b),
        },
    }
}
fn editor(
    bytes: &[u8],
    limits: FrameInteractionLimits,
) -> Result<EditorPageImage, SourcePageError> {
    let package = Package::open(bytes, bytes.len() as u64, Default::default(), &|| false).unwrap();
    let index = read(bytes);
    let original = serde_json::to_value(&index).unwrap();
    let mut q = request(&index);
    let size = index.page_size.unwrap();
    let denominator = (size.width.get() as u64).div_ceil(400);
    q.viewport.scale.denominator = denominator.try_into().unwrap();
    q.viewport.width = (size.width.get() as u64).div_ceil(denominator) as u32;
    q.viewport.height = (size.height.get() as u64).div_ceil(denominator) as u32;
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let result = source_editor_page::prepare(
        &PackageImages(&package),
        &index,
        &q,
        &mut NativeRaster,
        TextPageContext {
            manifest: &manifest,
            backend: &mut NativeShaper::default(),
        },
        EditorPageOptions {
            resources: options(),
            interaction: limits,
        },
        &|| false,
    )?
    .render(&mut NativeRaster, &|| false)?;
    let reference = source_resource_page::prepare(
        &package,
        &index,
        &q,
        &mut NativeRaster,
        Some(TextPageContext {
            manifest: &manifest,
            backend: &mut NativeShaper::default(),
        }),
        options(),
        &|| false,
    )?
    .render(&mut NativeRaster, &|| false)?;
    assert_eq!(
        result.pixels, reference.pixels,
        "editing preserves every raster byte"
    );
    assert_eq!(
        result.page.page().resources_sha256,
        reference.info.resources_sha256
    );
    assert_eq!(serde_json::to_value(&index).unwrap(), original);
    if let Some(dir) = std::env::var_os("MO_EDITOR_PAGE_FIXTURES") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        let name = index.source_sha256.to_string();
        std::fs::write(dir.join(format!("{name}.pptx")), bytes).unwrap();
        std::fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "operation": "prepare", "request": {
                    "input": {"kind": "pptx"}, "fonts": fonts,
                    "page": {"page": q, "imageSource": "embeddedSnapshot", "sampling": "nearest"}
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let document = mo_pptx::source::document::import_document(
            &package,
            "document:editor".to_owned().try_into().unwrap(),
            "resource:editor-source".to_owned().try_into().unwrap(),
            Default::default(),
            &|| false,
        )
        .unwrap();
        let mut changed = document;
        changed.title = "Edited retained title".into();
        let plan = mo_pptx::source::document::SourcePlan::new(
            &changed,
            &package,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let mut page = q.clone();
        page.expected_source_sha256 = plan.identity().clone();
        std::fs::write(dir.join(format!("{name}-retained.pptx")), bytes).unwrap();
        std::fs::write(
            dir.join(format!("{name}-retained.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "operation": "prepare", "request": {
                    "input": {"kind": "retained", "document": changed}, "fonts": fonts,
                    "page": {"page": page, "imageSource": "embeddedSnapshot", "sampling": "nearest"}
                }
            }))
            .unwrap(),
        )
        .unwrap();
        if index
            .surfaces
            .values()
            .any(|s| s.objects.iter().any(|o| o.table.is_some()))
        {
            use mo_presentation_model::{ObjectContent, SourceBindingProfile};
            for (version, profile) in [
                (1, SourceBindingProfile::PresentationmlRetainedFieldsV1),
                (2, SourceBindingProfile::PresentationmlRetainedFieldsV2),
            ] {
                let mut legacy = changed.clone();
                legacy.source_bindings.as_mut().unwrap().profile = profile;
                if version == 1 {
                    legacy.title.clear();
                }
                for (id, o) in &mut legacy.objects {
                    o.accessibility = Default::default();
                    let native = legacy
                        .source_bindings
                        .as_mut()
                        .unwrap()
                        .objects
                        .get_mut(id)
                        .unwrap();
                    let source = index.surfaces[&native.part]
                        .objects
                        .iter()
                        .find(|o| o.native_id == native.native_id)
                        .unwrap();
                    if source.table.is_some() {
                        let ObjectContent::RetainedSource { paragraphs, .. } = &mut o.content
                        else {
                            panic!()
                        };
                        paragraphs.clear();
                        native.runs.clear();
                    }
                }
                let plan = mo_pptx::source::document::SourcePlan::new(
                    &legacy,
                    &package,
                    Default::default(),
                    &|| false,
                )
                .unwrap();
                let mut page = q.clone();
                page.expected_source_sha256 = plan.identity().clone();
                std::fs::write(dir.join(format!("{name}-legacy-v{version}.pptx")), bytes).unwrap();
                std::fs::write(dir.join(format!("{name}-legacy-v{version}.json")),serde_json::to_vec_pretty(&serde_json::json!({
                    "operation":"prepare","request":{"input":{"kind":"retained","document":legacy},"fonts":fonts,
                    "page":{"page":page,"imageSource":"embeddedSnapshot","sampling":"nearest"}}
                })).unwrap()).unwrap();
            }
        }
    }
    Ok(result)
}

fn original_text(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if key == "text" && value.is_string() {
                    *value = serde_json::json!("AAA");
                } else {
                    original_text(value);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                original_text(value);
            }
        }
        serde_json::Value::String(s) if s == "Arial" => *s = FONT.into(),
        _ => (),
    }
}
#[test]
fn emit_authored_page_fixture_with_native_model_object_identity() {
    let mut input: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    original_text(&mut input);
    let document = serde_json::from_value(input["document"].clone()).unwrap();
    let defaults = serde_json::from_value(input["defaults"].clone()).unwrap();
    let plan =
        mo_pptx::AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
    if let Some(dir) = std::env::var_os("MO_EDITOR_PAGE_FIXTURES") {
        let dir = std::path::PathBuf::from(dir);
        let mut q = request(plan.declarations());
        q.viewport.width = 400;
        q.viewport.height = 225;
        q.viewport.scale.denominator = 30480;
        let material =
            include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
        let resources: Vec<_> = input["resourceBindings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id":r["resourceId"],"offset":r["byteOffset"],"byteLength":r["byteLength"]
                })
            })
            .collect();
        std::fs::create_dir_all(&dir).unwrap();
        for slide in 1..=2 {
            q.slide = format!("/ppt/slides/slide{slide}.xml");
            std::fs::write(dir.join(format!("author-{slide}.pptx")), material).unwrap();
            std::fs::write(dir.join(format!("author-{slide}.json")), serde_json::to_vec_pretty(&serde_json::json!({
                "operation":"prepare", "request":{
                    "input":{"kind":"author", "document":document,"defaults":defaults,"resources":resources},
                    "fonts":fonts(),"page":{"page":q,"imageSource":"embeddedSnapshot","sampling":"nearest"}
                }
            })).unwrap()).unwrap();
        }
    }
}

#[test]
fn emit_authored_table_with_physical_and_empty_cell_identity() {
    use mo_presentation_model::{Document, Inherited, ObjectContent, Stroke, TableCellBorders};
    let mut input: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    original_text(&mut input);
    let mut document: Document = serde_json::from_value(input["document"].clone()).unwrap();
    let defaults = serde_json::from_value(input["defaults"].clone()).unwrap();
    {
        // The source fixture deliberately has conflicting shared borders.
        // Preserve that rejection before probing a separate agreeing table.
        let plan =
            mo_pptx::AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
        let images =
            mo_pptx::source::images::AuthorImages::new(&plan, &mo_pptx::NoResources, &|| false)
                .unwrap();
        let fonts = fonts();
        let manifest =
            PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
        let mut q = request(plan.declarations());
        q.viewport.width = 400;
        q.viewport.height = 225;
        q.viewport.scale.denominator = 30480;
        let result = source_editor_page::prepare(
            &images,
            plan.declarations(),
            &q,
            &mut NativeRaster,
            TextPageContext {
                manifest: &manifest,
                backend: &mut NativeShaper::default(),
            },
            EditorPageOptions {
                resources: options(),
                interaction: Default::default(),
            },
            &|| false,
        );
        let Err(error) = result else {
            panic!("conflicting table borders must remain a diagnostic");
        };
        assert!(error.to_string().contains("TableBorderConflict"), "{error}");
    }
    for object in document.objects.values_mut() {
        if let ObjectContent::Table { table } = &mut object.content {
            for cell in table.rows.iter_mut().flat_map(|r| &mut r.cells) {
                cell.style.borders = TableCellBorders {
                    left: Inherited::Value(Stroke::None {}),
                    right: Inherited::Value(Stroke::None {}),
                    top: Inherited::Value(Stroke::None {}),
                    bottom: Inherited::Value(Stroke::None {}),
                    top_left_to_bottom_right: Inherited::Value(Stroke::None {}),
                    bottom_left_to_top_right: Inherited::Value(Stroke::None {}),
                };
            }
        }
    }
    let plan =
        mo_pptx::AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
    if let Some(dir) = std::env::var_os("MO_EDITOR_PAGE_FIXTURES") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut q = request(plan.declarations());
        q.viewport.width = 400;
        q.viewport.height = 225;
        q.viewport.scale.denominator = 30480;
        std::fs::write(dir.join("author-table.pptx"), []).unwrap();
        std::fs::write(dir.join("author-table.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "operation":"prepare","request":{"input":{"kind":"author","document":document,"defaults":defaults,"resources":[]},
            "fonts":fonts(),"page":{"page":q,"imageSource":"embeddedSnapshot","sampling":"nearest"}}
        })).unwrap()).unwrap();
    }
}
fn mapped(page: &SourceEditorPage, frame: usize, point: Point) -> Point {
    let text = &page.page().text.as_ref().unwrap().texts[frame];
    let placement = page.page().page.bindings[text.binding as usize]
        .placement
        .as_ref()
        .unwrap();
    placement
        .affine
        .map(Point {
            x: point.x.checked_sub(placement.anchor.x).unwrap(),
            y: point.y.checked_sub(placement.anchor.y).unwrap(),
        })
        .unwrap()
        .point
}
fn near(a: Point, b: Point, tolerance: Fixed) {
    assert!(a.x.raw().abs_diff(b.x.raw()) <= tolerance.raw() as u128);
    assert!(a.y.raw().abs_diff(b.y.raw()) <= tolerance.raw() as u128);
}
fn pick_query(e: &SourceEditorPage, page: Point, max_hits: u32) -> PagePickQuery {
    let v = &e.page().page.raster.viewport;
    let scale = |p: Fixed| {
        Fixed::from_raw(p.raw() * i128::from(v.scale.numerator) / i128::from(v.scale.denominator))
    };
    PagePickQuery {
        device: mo_raster::picking::DevicePickQuery {
            point: Point {
                x: scale(page.x),
                y: scale(page.y),
            },
            radius: Fixed::ZERO,
        },
        max_hits,
    }
}
fn frame_center(e: &SourceEditorPage, frame: usize) -> Point {
    let r = e.page().text.as_ref().unwrap().texts[frame]
        .frame
        .region
        .outer;
    mapped(
        e,
        frame,
        Point {
            x: Fixed::from_raw((r.min.x.raw() + r.max.x.raw()) / 2),
            y: Fixed::from_raw((r.min.y.raw() + r.max.y.raw()) / 2),
        },
    )
}
#[test]
fn rotated_flipped_nested_page_queries_match_rendered_frame_and_resource_pixels() {
    for (orientation, nested) in [
        ("", false),
        ("rot=\"5400000\"", false),
        ("rot=\"2700000\" flipH=\"1\"", false),
        ("rot=\"1800000\" flipV=\"1\"", true),
    ] {
        let s = shape(42, 200000, 100000, orientation, &colored("AAA", "123456"));
        let s = if nested {
            group(
                90,
                &transform(
                    "rot=\"1200000\" flipH=\"1\"",
                    [100000, 100000, 1200000, 900000],
                    [0, 0, 1600000, 1200000],
                ),
                "<a:noFill/>",
                &group(
                    91,
                    &transform(
                        "rot=\"600000\"",
                        [0, 0, 1400000, 800000],
                        [0, 0, 1600000, 1200000],
                    ),
                    "<a:noFill/>",
                    &s,
                ),
            )
        } else {
            s
        };
        let bytes = image_fixture(&(picture(43, "", &blip("owned-image", "", STRETCH), "") + &s));
        let image = editor(&bytes, Default::default()).unwrap();
        let e = &image.page;
        assert_eq!(e.page().images.decoded.len(), 1);
        assert_eq!(e.page().text.as_ref().unwrap().texts.len(), 1);
        let picked = e
            .pick(
                &[pick_query(e, frame_center(e, 0), 256)],
                &mut NativeRaster,
                &|| false,
            )
            .unwrap();
        assert_eq!(picked[0].hits[0].object.native_id, 42);
        assert_eq!(picked[0].hits[0].text_frame, Some(0));
        let answers = e.query(&[caret(0, 1), select(0, 0, 3)], &|| false).unwrap();
        let PageTextQueryResult::Caret { caret: c, .. } = &answers[0] else {
            panic!()
        };
        let movement = e
            .query(
                &[PageTextQuery {
                    frame: 0,
                    action: PageTextAction::Move {
                        position: position(0),
                        movement: CaretMove::NextGrapheme,
                        preferred_x: None,
                    },
                }],
                &|| false,
            )
            .unwrap();
        let PageTextQueryResult::Moved {
            caret: moved,
            exhausted,
            preferred_x,
            ..
        } = &movement[0]
        else {
            panic!()
        };
        assert!(!exhausted);
        assert!(preferred_x.is_none());
        assert_eq!(moved.edge, c.edge);
        assert_eq!(moved.visible, c.visible);
        assert_eq!(moved.local.caret.position.affinity, Affinity::Upstream);
        let resolved = e
            .query(
                &[PageTextQuery {
                    frame: 0,
                    action: PageTextAction::Caret {
                        position: FrameTextPosition {
                            paragraph: moved.local.paragraph,
                            position: moved.local.caret.position,
                        },
                    },
                }],
                &|| false,
            )
            .unwrap();
        let PageTextQueryResult::Caret {
            caret: resolved, ..
        } = &resolved[0]
        else {
            panic!()
        };
        assert_eq!(
            serde_json::to_value(moved).unwrap(),
            serde_json::to_value(resolved).unwrap()
        );
        let local = c.local.caret.edge;
        near(
            c.edge[0],
            mapped(
                e,
                0,
                Point {
                    x: local.x,
                    y: local.top,
                },
            ),
            c.coordinate_error_bound,
        );
        near(
            c.edge[1],
            mapped(
                e,
                0,
                Point {
                    x: local.x,
                    y: local.bottom,
                },
            ),
            c.coordinate_error_bound,
        );
        assert_eq!(c.visible, Some(c.edge));
        let center = Point {
            x: c.edge[0]
                .x
                .checked_add(c.edge[1].x)
                .unwrap()
                .half()
                .unwrap(),
            y: c.edge[0]
                .y
                .checked_add(c.edge[1].y)
                .unwrap()
                .half()
                .unwrap(),
        };
        let hit = e
            .query(
                &[PageTextQuery {
                    frame: 0,
                    action: PageTextAction::Hit { point: center },
                }],
                &|| false,
            )
            .unwrap();
        let PageTextQueryResult::Hit {
            caret: Some(h),
            inside,
            ..
        } = &hit[0]
        else {
            panic!()
        };
        assert!(*inside);
        assert_eq!(h.local.caret.position.scalar_offset, 1);
        let PageTextQueryResult::Selection { fragments, .. } = &answers[1] else {
            panic!()
        };
        assert_eq!(fragments.len(), 3);
        for fragment in fragments {
            let r = fragment.local.fragment.bounds;
            for (actual, local) in fragment.quad.into_iter().zip([
                r.min,
                Point {
                    x: r.max.x,
                    y: r.min.y,
                },
                r.max,
                Point {
                    x: r.min.x,
                    y: r.max.y,
                },
            ]) {
                near(actual, mapped(e, 0, local), fragment.coordinate_error_bound);
            }
            assert_eq!(fragment.visible, Some(fragment.quad));
        }
        if orientation.contains("2700000") || nested {
            assert_ne!(fragments[0].quad[0].y, fragments[0].quad[1].y);
            assert_ne!(c.edge[0].x, c.edge[1].x);
        }
    }
}
#[test]
fn native_clips_keep_actual_rotated_selection_quads_and_hidden_carets() {
    let s = shape(
        42,
        300000,
        300000,
        "rot=\"2700000\"",
        &(colored("AAAA", "123456") + "<a:br/>" + &colored("AAAA", "123456")),
    )
    .replace(
        "cx=\"1000000\" cy=\"600000\"",
        "cx=\"240000\" cy=\"240000\"",
    )
    .replace(
        "<a:bodyPr ",
        "<a:bodyPr wrap=\"none\" horzOverflow=\"clip\" vertOverflow=\"clip\" ",
    );
    let image = editor(&fixture(&s), Default::default()).unwrap();
    let e = &image.page;
    let a = e.query(&[select(0, 0, 9), caret(0, 9)], &|| false).unwrap();
    let PageTextQueryResult::Selection { fragments, .. } = &a[0] else {
        panic!()
    };
    assert!(fragments.iter().any(|f| f.visible.is_none()));
    let fragment = fragments.iter().find(|f| f.visible.is_some()).unwrap();
    let clip = fragment.local.visible.unwrap();
    assert!(clip.min.x >= f(0) && clip.max.x <= f(240000));
    assert!(clip.min.y >= f(0) && clip.max.y <= f(240000));
    assert_ne!(fragment.visible.unwrap(), fragment.quad);
    let PageTextQueryResult::Caret { caret: c, .. } = &a[1] else {
        panic!()
    };
    assert!(c.visible.is_none());
    let hit = e
        .query(
            &[PageTextQuery {
                frame: 0,
                action: PageTextAction::Hit { point: c.edge[0] },
            }],
            &|| false,
        )
        .unwrap();
    assert!(matches!(
        hit[0],
        PageTextQueryResult::Hit { inside: false, .. }
    ));
}
#[test]
fn page_unicode_offsets_preserve_graphemes_and_bidi_affinity() {
    let s = shape(
        42,
        0,
        0,
        "rot=\"1800000\"",
        &colored("A\u{0301} 😀אבA", "123456"),
    )
    .replace(
        &format!("<a:latin typeface=\"{FONT}\"/>"),
        &format!(
            "<a:latin typeface=\"{FONT}\"/><a:ea typeface=\"{FONT}\"/><a:cs typeface=\"{FONT}\"/>"
        ),
    );
    let bytes = fixture(&s);
    let image = editor(&bytes, Default::default()).unwrap();
    let page = &image.page;
    let map = &page.paragraphs(0).unwrap()[0];
    assert!(!map.boundaries.iter().any(|b| b.scalar_offset == 1));
    assert!(
        map.boundaries
            .iter()
            .any(|b| b.utf16_offset > b.scalar_offset)
    );
    assert!(page.query(&[caret(0, 1)], &|| false).is_err());
    for boundary in &map.boundaries {
        for affinity in [Affinity::Upstream, Affinity::Downstream] {
            let mut p = position(boundary.scalar_offset);
            p.position.affinity = affinity;
            let result = page
                .query(
                    &[PageTextQuery {
                        frame: 0,
                        action: PageTextAction::Caret { position: p },
                    }],
                    &|| false,
                )
                .unwrap();
            let PageTextQueryResult::Caret { caret, .. } = &result[0] else {
                panic!()
            };
            assert_eq!(caret.local.caret.boundary, *boundary);
        }
    }
    let text = &page.page().text.as_ref().unwrap().texts[0];
    assert_eq!(text.frame.inputs[0].text, "A\u{0301} 😀אבA");
}
#[test]
fn page_query_failures_are_atomic_and_budgets_cover_all_text_frames() {
    let bytes = fixture(
        &(shape(42, 0, 0, "", &colored("AA", "123456"))
            + &shape(43, 100000, 600000, "", &colored("AA", "123456"))),
    );
    let image = editor(
        &bytes,
        FrameInteractionLimits {
            max_selection_fragments: 3,
            max_query_work: 8,
            ..Default::default()
        },
    )
    .unwrap();
    let e = &image.page;
    for queries in [
        vec![caret(0, 0), caret(2, 0)],
        vec![caret(0, 0), caret(1, 99)],
        vec![caret(0, 0); 65],
    ] {
        assert!(e.query(&queries, &|| false).is_err());
    }
    assert!(e.query(&[caret(0, 0)], &|| true).is_err());
    assert!(e.query(&[caret(0, 0)], &|| false).is_ok());
    assert!(e.query(&[caret(1, 0)], &|| false).is_ok());
    let error = e.query(&[caret(0, 0), caret(1, 0)], &|| false).unwrap_err();
    assert!(error.to_string().contains("query work"), "{error}");
    let image = editor(
        &bytes,
        FrameInteractionLimits {
            max_selection_fragments: 3,
            ..Default::default()
        },
    )
    .unwrap();
    let e = &image.page;
    let query = [
        select(0, 0, 2),
        PageTextQuery {
            frame: 1,
            action: PageTextAction::Move {
                position: position(0),
                movement: CaretMove::NextGrapheme,
                preferred_x: None,
            },
        },
        PageTextQuery {
            frame: 0,
            action: PageTextAction::Move {
                position: position(1),
                movement: CaretMove::Down,
                preferred_x: Some(Fixed::ZERO),
            },
        },
    ];
    let calls = std::cell::Cell::new(0usize);
    e.query(&query, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let at = std::cell::Cell::new(0usize);
        assert!(
            e.query(&query, &|| {
                at.set(at.get() + 1);
                at.get() == stop
            })
            .is_err()
        );
        assert!(e.query(&[caret(0, 0)], &|| false).is_ok());
    }
    assert!(e.query(&[select(0, 0, 2)], &|| false).is_ok());
    assert!(e.query(&[select(1, 0, 2)], &|| false).is_ok());
    let error = e
        .query(&[select(0, 0, 2), select(1, 0, 2)], &|| false)
        .unwrap_err();
    assert!(error.to_string().contains("selection fragments"), "{error}");
    for limits in [
        FrameInteractionLimits {
            max_cells: 3,
            ..Default::default()
        },
        FrameInteractionLimits {
            max_lines: 1,
            ..Default::default()
        },
    ] {
        assert!(editor(&bytes, limits).is_err());
    }
}
#[test]
fn native_table_cells_keep_their_own_paragraph_and_empty_caret_bindings() {
    let original = tables::fixture(false);
    // The source fixture intentionally declares incompatible shared borders.
    // Editing must preserve that diagnostic rather than paint a partial table.
    assert!(matches!(
        editor(&original, Default::default()),
        Err(SourcePageError::Mapping { .. })
    ));
    // This text probe uses explicit, agreeing no-fill borders on every cell.
    let bytes = rewrite(&original, SLIDE, |mut xml| {
        for edge in ["lnL", "lnR", "lnT", "lnB", "lnTlToBr", "lnBlToTr"] {
            let open = format!("<a:{edge}");
            let close = format!("</a:{edge}>");
            let mut from = 0;
            while let Some(start) = xml[from..].find(&open).map(|n| n + from) {
                let end = xml[start..].find(&close).unwrap() + start + close.len();
                let replacement = format!("<a:{edge}><a:noFill/></a:{edge}>");
                xml.replace_range(start..end, &replacement);
                from = start + replacement.len();
            }
        }
        xml
    });
    let image = editor(&bytes, Default::default()).unwrap();
    let e = &image.page;
    let text = e.page().text.as_ref().unwrap();
    assert_eq!(
        text.texts.len(),
        6,
        "covered cells do not create editable duplicate text"
    );
    let answers = e
        .query(
            &(0..6).map(|frame| caret(frame, 0)).collect::<Vec<_>>(),
            &|| false,
        )
        .unwrap();
    for (i, answer) in answers.iter().enumerate() {
        let PageTextQueryResult::Caret { frame, caret: c } = answer else {
            panic!()
        };
        assert_eq!(*frame as usize, i);
        assert_eq!(c.local.paragraph, 0);
        assert_eq!(c.local.caret.position.scalar_offset, 0);
        assert!(c.visible.is_some());
        let picked = e
            .pick(
                &[pick_query(e, frame_center(e, i), 256)],
                &mut NativeRaster,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            picked[0].hits.len(),
            1,
            "table deduplicates its native object"
        );
        assert_eq!(picked[0].hits[0].text_frame, Some(i as u32));
    }
    assert_ne!(
        serde_json::to_value(&answers[0]).unwrap(),
        serde_json::to_value(&answers[1]).unwrap()
    );
}

#[test]
fn object_picking_uses_curved_fill_and_reverse_paint_order_with_bounded_results() {
    let back = receiver(
        40,
        [0, 0, 1600000, 1200000],
        "",
        "<a:solidFill><a:srgbClr val=\"ABCDEF\"/></a:solidFill>",
    );
    let ellipse = receiver(
        41,
        [200000, 200000, 1000000, 600000],
        "",
        "<a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill>",
    )
    .replace("prst=\"rect\"", "prst=\"ellipse\"");
    let image = editor(&fixture(&(back + &ellipse)), Default::default()).unwrap();
    let e = &image.page;
    let middle = Point {
        x: f(700000),
        y: f(500000),
    };
    let queries = [
        pick_query(e, middle, 256),
        pick_query(e, middle, 1),
        pick_query(
            e,
            Point {
                x: f(210000),
                y: f(210000),
            },
            256,
        ),
    ];
    let picked = e.pick(&queries, &mut NativeRaster, &|| false).unwrap();
    assert_eq!(
        picked[0]
            .hits
            .iter()
            .map(|h| h.object.native_id)
            .collect::<Vec<_>>(),
        [41, 40]
    );
    assert!(!picked[0].truncated);
    assert_eq!(picked[1].hits.len(), 1);
    assert!(picked[1].truncated);
    assert_eq!(
        picked[2]
            .hits
            .iter()
            .map(|h| h.object.native_id)
            .collect::<Vec<_>>(),
        [40]
    );
    assert_eq!(
        e.objects().iter().map(|o| o.native_id).collect::<Vec<_>>(),
        [40, 41]
    );
}

#[test]
fn empty_text_frame_retains_unused_native_clip_and_fractional_page_boundary() {
    let empty = shape(42, 1500000, 900000, "rot=\"2700000\"", "")
        .replace(
            "<a:solidFill><a:srgbClr val=\"F4EADC\"/></a:solidFill>",
            "<a:noFill/>",
        )
        .replace(
            "<a:bodyPr ",
            "<a:bodyPr horzOverflow=\"clip\" vertOverflow=\"clip\" ",
        );
    let bytes = rewrite(&fixture(&empty), "/ppt/presentation.xml", |s| {
        s.replace(
            "cx=\"1600000\" cy=\"1200000\"",
            "cx=\"1600001\" cy=\"1200001\"",
        )
    });
    let image = editor(&bytes, Default::default()).unwrap();
    let e = &image.page;
    let text = &e.page().text.as_ref().unwrap().texts[0];
    assert!(text.frame.glyphs.is_empty());
    assert!(text.frame.clip.is_some());
    // Search only the geometric text rectangle, using the same independently
    // forward-mapped local points as the existing native caret qualification.
    let mut inside = 0;
    for y in 0..10 {
        for x in 0..10 {
            let p = mapped(
                e,
                0,
                Point {
                    x: f(x * 100000 + 50000),
                    y: f(y * 60000 + 30000),
                },
            );
            let picked = e
                .pick(&[pick_query(e, p, 256)], &mut NativeRaster, &|| false)
                .unwrap();
            if !picked[0].hits.is_empty() {
                inside += 1;
                assert_eq!(picked[0].hits[0].object.native_id, 42);
                assert_eq!(picked[0].hits[0].text_frame, Some(0));
                assert!(
                    p.x >= Fixed::ZERO
                        && p.x <= f(1600001)
                        && p.y >= Fixed::ZERO
                        && p.y <= f(1200001)
                );
            }
        }
    }
    assert!(inside > 0 && inside < 100);
    let v = &e.page().page.raster.viewport;
    let q = PagePickQuery {
        device: mo_raster::picking::DevicePickQuery {
            point: Point {
                x: Fixed::from_raw((i128::from(v.width) << 32) - (1 << 16)),
                y: f(250),
            },
            radius: f(32),
        },
        max_hits: 256,
    };
    assert!(
        e.pick(&[q], &mut NativeRaster, &|| false).unwrap()[0]
            .hits
            .is_empty(),
        "pointer tolerance cannot extend the fractional page clip"
    );
}

#[test]
fn page_pick_preflight_and_cancellation_do_not_invoke_a_component() {
    struct Unused;
    impl mo_raster::RasterBackend for Unused {
        fn raster(&mut self, _: &[u32]) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
            panic!("unexpected raster call")
        }
        fn pick(
            &mut self,
            _: &[u32],
        ) -> Result<mo_raster::picking::PickingReply, mo_raster::RasterError> {
            panic!("unexpected picking call")
        }
        fn invalidate(&mut self) {
            panic!("preflight cannot invalidate an untouched component")
        }
    }
    let image = editor(&fixture(&shape(42, 0, 0, "", "")), Default::default()).unwrap();
    let e = &image.page;
    let good = pick_query(e, frame_center(e, 0), 256);
    assert!(e.pick(&[], &mut Unused, &|| false).unwrap().is_empty());
    for max_hits in [0, 257] {
        assert!(
            e.pick(
                &[PagePickQuery {
                    max_hits,
                    ..good.clone()
                }],
                &mut Unused,
                &|| false
            )
            .is_err()
        );
    }
    assert!(
        e.pick(&vec![good.clone(); 65], &mut Unused, &|| false)
            .is_err()
    );
    let mut invalid = good.clone();
    invalid.device.radius = f(-1);
    assert!(e.pick(&[invalid], &mut Unused, &|| false).is_err());
    assert!(matches!(
        e.pick(&[good], &mut Unused, &|| true),
        Err(SourcePageError::Raster(mo_raster::RasterError::Cancelled))
    ));
}
