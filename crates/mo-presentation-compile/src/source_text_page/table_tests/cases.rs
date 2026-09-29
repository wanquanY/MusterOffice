use super::*;

pub(super) fn edit_cell(bytes: &[u8], cell: usize, f: impl FnOnce(&str) -> String) -> Vec<u8> {
    rewrite(bytes, SLIDE, |mut s| {
        let start = s
            .match_indices("<a:tc")
            .filter(|(at, _)| matches!(s.as_bytes()[at + 5], b' ' | b'>'))
            .nth(cell)
            .unwrap()
            .0;
        let end = start + s[start..].find("</a:tc>").unwrap() + 7;
        s.replace_range(start..end, &f(&s[start..end]));
        s
    })
}
pub(super) fn two_tables(bytes: &[u8]) -> Vec<u8> {
    rewrite(bytes, SLIDE, |mut s| {
        let start = s.find("<p:graphicFrame>").unwrap();
        let end = start + s[start..].find("</p:graphicFrame>").unwrap() + 17;
        let frame = &s[start..end];
        let id_at = frame.find("<p:cNvPr id=\"").unwrap() + 13;
        let id_end = id_at + frame[id_at..].find('"').unwrap();
        let copy = format!("{}990{}", &frame[..id_at], &frame[id_end..]);
        s.insert_str(end, &copy);
        s
    })
}
fn assert_rejected(index: &SourceIndex, limits: TextPageLimits, second_preflight: bool) {
    let q = request(index);
    let b = bindings(index, &q, 0);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    let mut c = Compiler::new(&manifest, &mut shaper, limits);
    let result = if second_preflight {
        c.preflight(index, &q, b[..1].iter(), &|| false).unwrap();
        c.preflight(index, &q, b[1..].iter(), &|| false)
    } else {
        c.preflight(index, &q, b.iter(), &|| false)
    };
    assert!(result.is_err());
    assert!(c.finish().is_err());
    assert_eq!(shaper.calls, 0);
}

#[test]
fn limits_are_shared_across_tables_and_repeated_preflight() {
    let bytes = two_tables(&fixture(false));
    let index = read(&bytes);
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    assert_eq!(b.len(), 2);
    let (content, scene) = compile(&index, &q, &b);
    assert_eq!(content.texts.len(), 12);
    for text in &content.texts[6..] {
        assert_eq!(text.binding, 1);
        assert_eq!(text.frame.text.object.native_id, 990);
    }
    let retained = retained::RetainedText::new(content, &b, &|| false).unwrap();
    assert_eq!(retained.frames(), 12);
    let visible = BTreeSet::from([(SLIDE.into(), Some(990))]);
    let mut p = retained.painter(&visible);
    let mut builder = SceneBuilder::new();
    p.append(1, &b[1], &mut builder, &q.viewport, &|| false)
        .unwrap();
    let (_, capacity) = p.finish().unwrap();
    assert_eq!(capacity.frames.len(), 6);
    assert!(capacity.frames.iter().all(|f| f.object.native_id == 990));
    assert!(!scene.instances.is_empty());
    for repeated in [false, true] {
        assert_rejected(
            &index,
            TextPageLimits {
                max_frames: 11,
                ..Default::default()
            },
            repeated,
        );
        assert_rejected(
            &index,
            TextPageLimits {
                tables: crate::source_table::TableGeometryLimits {
                    grid: mo_presentation_source::source::table::grid::NativeTableGridLimits {
                        max_cells: 17,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            repeated,
        );
        assert_rejected(
            &index,
            TextPageLimits {
                tables: crate::source_table::TableGeometryLimits {
                    grid: mo_presentation_source::source::table::grid::NativeTableGridLimits {
                        max_steps: 41,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            repeated,
        );
    }
}

#[test]
fn text_work_limits_and_cancellation_cannot_publish_partial_cells() {
    let index = read(&fixture(false));
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    let (content, _) = compile(&index, &q, &b);
    let first = &content.texts[0].frame.work;
    let total = &content.text_work;
    assert!(first.component_calls < total.component_calls);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let limits = [
        SourceFrameLimits {
            max_component_calls: total.component_calls - 1,
            ..Default::default()
        },
        SourceFrameLimits {
            max_font_upload_bytes: total.font_upload_bytes - 1,
            ..Default::default()
        },
        SourceFrameLimits {
            max_request_words: total.request_words - 1,
            ..Default::default()
        },
        SourceFrameLimits {
            max_glyphs: total.glyphs - 1,
            ..Default::default()
        },
        SourceFrameLimits {
            max_path_commands: total.path_commands - 1,
            ..Default::default()
        },
    ];
    // A failed native component is permanently invalidated process-wide.
    // Exercise each budget in its own worker process, without a reset API.
    let selected = std::env::var("MO_TABLE_TEXT_BUDGET_CASE").ok();
    for (case, limit) in limits.into_iter().enumerate() {
        if let Some(selected) = &selected {
            if *selected != case.to_string() {
                continue;
            }
        } else {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "source_text_page::table_tests::cases::text_work_limits_and_cancellation_cannot_publish_partial_cells",
                    "--test-threads=1",
                ])
                .env("MO_TABLE_TEXT_BUDGET_CASE", case.to_string())
                .output().unwrap();
            assert!(
                result.status.success(),
                "case {case}: {}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            continue;
        }
        let mut shaper = Shaper::default();
        let mut c = Compiler::new(
            &manifest,
            &mut shaper,
            TextPageLimits {
                work: limit,
                ..Default::default()
            },
        );
        c.preflight(&index, &q, b.iter(), &|| false).unwrap();
        let mut builder = SceneBuilder::new();
        assert!(
            c.append(0, &b[0], &mut builder, &q.viewport, &|| false)
                .is_err()
        );
        assert!(
            !builder.scene.instances.is_empty(),
            "earlier cell really computed"
        );
        assert!(
            c.append(0, &b[0], &mut builder, &q.viewport, &|| false)
                .is_err()
        );
        assert!(
            c.preflight(&index, &q, std::iter::empty(), &|| false)
                .is_err()
        );
        assert!(c.finish().is_err());
        assert!(shaper.calls > 0 && shaper.calls <= limit.max_component_calls as usize);
    }
    if selected.is_some() {
        return;
    }
    let visible = BTreeSet::from([(SLIDE.into(), b[0].location.object)]);
    let retained = retained::RetainedText::new(content, &b, &|| false).unwrap();
    let calls = Cell::new(0);
    let mut complete = retained.painter(&visible);
    complete
        .append(0, &b[0], &mut SceneBuilder::new(), &q.viewport, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    complete.finish().unwrap();
    let midway = calls.get() / 2;
    calls.set(0);
    let mut p = retained.painter(&visible);
    let mut builder = SceneBuilder::new();
    assert!(
        p.append(0, &b[0], &mut builder, &q.viewport, &|| {
            calls.set(calls.get() + 1);
            calls.get() > midway
        })
        .is_err()
    );
    assert!(!builder.scene.instances.is_empty());
    assert!(
        p.append(0, &b[0], &mut builder, &q.viewport, &|| false)
            .is_err()
    );
    assert!(p.finish().is_err());
    // The immutable retained plan remains usable by a fresh transaction.
    let mut p = retained.painter(&visible);
    p.append(0, &b[0], &mut SceneBuilder::new(), &q.viewport, &|| false)
        .unwrap();
    p.finish().unwrap();
}

#[test]
fn covered_payload_missing_body_and_original_diagnostic_locations_survive() {
    let bytes = edit_cell(&fixture(false), 1, |s| {
        s.replace("<a:r>", "<a:r><a:rPr cap=\"all\"/>")
    });
    let bytes = edit_cell(&bytes, 2, |s| {
        let start = s.find("<a:txBody>").unwrap();
        let end = s.find("</a:txBody>").unwrap() + 11;
        format!("{}{}", &s[..start], &s[end..])
    });
    let index = read(&bytes);
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    let before = serde_json::to_vec(&index).unwrap();
    let (content, scene) = compile(&index, &q, &b);
    assert_eq!(content.texts.len(), 5);
    assert!(!content.texts.iter().any(|t| t.frame.text.cell == Some(cell(0, 1)) || t.frame.text.cell == Some(cell(0, 2))));
    assert_eq!(before, serde_json::to_vec(&index).unwrap());
    let pixels = raster(&q, scene.clone());
    evidence("missing-body-covered", &bytes, &content, &scene, &pixels);

    let bytes = edit_cell(&fixture(false), 5, |s| {
        s.replace(
            "<a:r>",
            "<a:r><a:rPr><a:solidFill><a:sysClr val=\"windowText\"/></a:solidFill></a:rPr>",
        )
    });
    let index = read(&bytes);
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    let mut c = Compiler::new(&manifest, &mut shaper, Default::default());
    let error = c.preflight(&index, &q, b.iter(), &|| false).unwrap_err();
    let SourcePageError::AtObject { location, error } = error else {
        panic!("{error:?}")
    };
    assert_eq!(location.object, b[0].location.object);
    let SourcePageError::TextPaint(paint::TextPaintError::AtRun { location, .. }) = *error else {
        panic!("{error:?}")
    };
    assert_eq!(location.paragraph, 6);
    assert!(c.finish().is_err());
    assert_eq!(shaper.calls, 0);
}

#[test]
fn cell_colors_no_fill_and_decorations_use_independent_retained_clips() {
    let mut bytes = rewrite(&fixture(false), SLIDE, |s| {
        s.replace("horzOverflow=\"overflow\"", "horzOverflow=\"clip\"")
    });
    for (i, color) in [(0, "CC2200"), (2, "008844"), (5, "0022CC"), (6, "660088")] {
        bytes = edit_cell(&bytes, i, |s| {
            s.replace("<a:r>", &format!("<a:r><a:rPr u=\"sng\" strike=\"sngStrike\"><a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill></a:rPr>")).replace("A A", &"A".repeat(32))
        });
    }
    bytes = edit_cell(&bytes, 7, |s| {
        s.replace("<a:r>", "<a:r><a:rPr><a:noFill/></a:rPr>")
    });
    let index = read(&bytes);
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    let (content, scene) = compile(&index, &q, &b);
    assert_eq!(content.texts.len(), 6);
    assert!(
        content
            .texts
            .iter()
            .all(|t| t.frame.clip.as_ref().is_some_and(|c| c.horizontal))
    );
    assert!(!content.decoration_sources.is_empty());
    assert!(!content.texts[4].frame.glyphs.is_empty());
    assert!(content.texts[4].clusters.iter().all(|c| c.rgba.is_none()));
    assert!(content.text_sources.iter().all(|s| s.text_binding != 4));
    let colors: BTreeSet<_> = content
        .texts
        .iter()
        .flat_map(|f| f.clusters.iter().filter_map(|c| c.rgba))
        .collect();
    assert_eq!(colors.len(), 4);
    let retained = retained::RetainedText::new(content.clone(), &b, &|| false).unwrap();
    let visible = BTreeSet::from([(SLIDE.into(), b[0].location.object)]);
    for motion in [0, 5] {
        let sample = bindings(&index, &q, motion);
        let (_, fresh) = compile(&index, &q, &sample);
        let mut p = retained.painter(&visible);
        let mut builder = SceneBuilder::new();
        p.append(0, &sample[0], &mut builder, &q.viewport, &|| false)
            .unwrap();
        assert_eq!(p.finish().unwrap().0.component_calls, 0);
        assert_eq!(
            serde_json::to_vec(&builder.scene).unwrap(),
            serde_json::to_vec(&fresh).unwrap()
        );
        assert_eq!(raster(&q, builder.scene), raster(&q, fresh));
    }
    let pixels = raster(&q, scene.clone());
    evidence("colors-decorations", &bytes, &content, &scene, &pixels);
}
