//! Real shared NativeShaper execution from inspected native table cell bodies.
#[allow(dead_code)]
#[path = "../../../tools/test-support/table_text.rs"]
mod support;
use mo_harfbuzz_sys::NativeShaper;
use mo_pptx::source::{table::SourceCellAddress, text::cascade::TextStyleOrigin};
use mo_presentation_compile::source_text::*;
use mo_text::manifest::*;
use support::*;

fn compiler(i: &mo_pptx::source::SourceIndex) -> TableTextCompiler<'_> {
    match TableTextCompiler::bind(
        i,
        &i.source_sha256,
        &target(i),
        Default::default(),
        Default::default(),
        &|| false,
    )
    .unwrap()
    {
        TableTextPreparation::Prepared { table } => table,
        TableTextPreparation::Unresolved { issue } => panic!("{issue:?}"),
    }
}
fn prepared(table: &TableTextCompiler<'_>, row: u32, column: u32) -> PreparedSourceText {
    match table
        .prepare(
            SourceCellAddress { row, column },
            Default::default(),
            &|| false,
        )
        .unwrap()
    {
        SourceTextPreparation::Prepared { text } => text,
        other => panic!("{other:?}"),
    }
}
fn flow() -> SourceGlyphFlow {
    SourceGlyphFlow {
        width: mo_common::Emu::new(1_000_000),
        spacing: mo_text::geometry::LineSpacing::Natural,
        overflow: mo_text::flow::OverflowPolicy::EmergencyGrapheme,
        bounds_tolerance: mo_geometry::Fixed::from_raw(1 << 26),
    }
}
#[test]
fn physical_cells_share_the_real_native_text_engine_and_keep_original_paragraphs() {
    let bytes = fixture(false);
    let i = read(&bytes);
    let table = compiler(&i);
    let request: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))
    .unwrap();
    let manifest = PreparedManifest::load(
        &request.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let mut backend = NativeShaper::default();
    let ordinary = read(&support::bytes(
        "<a:p><a:r><a:rPr sz=\"2200\" lang=\"en\"/><a:t>A A</a:t></a:r><a:endParaRPr sz=\"2200\" lang=\"en\"/></a:p>",
    ));
    let SourceTextPreparation::Prepared {
        text: ordinary_text,
    } = prepare(
        &ordinary,
        &ordinary.source_sha256,
        &target(&ordinary),
        Default::default(),
        &|| false,
    )
    .unwrap()
    else {
        panic!()
    };
    let reference = ordinary_text
        .shape_paragraph(0, &manifest, &mut backend, &|| false)
        .unwrap()
        .computation;
    let reference_paths = ordinary_text
        .paragraph_paths(0, flow(), &manifest, &mut backend, &|| false)
        .unwrap()
        .computation;
    let mut outputs = vec![];
    for row in 0..3 {
        for column in 0..3 {
            let text = prepared(&table, row, column);
            let flat = row * 3 + column + u32::from(row * 3 + column > 4);
            assert_eq!(text.source().paragraph_start, flat);
            for (local, plan) in text.paragraphs().iter().enumerate() {
                assert!(plan.fonts.iter().all(|f| matches!(
                    f.font.declared_by.origin,
                    TextStyleOrigin::TableStyle { .. }
                )));
                let result = text
                    .shape_paragraph(local as u32, &manifest, &mut backend, &|| false)
                    .unwrap();
                assert_eq!(result.paragraph, flat + local as u32);
                assert_eq!(result.source_ordinal, plan.source_ordinal);
                if (row, column) != (2, 2) {
                    assert_eq!(
                        serde_json::to_value(&result.computation).unwrap(),
                        serde_json::to_value(&reference).unwrap()
                    );
                }
                let paths = text
                    .paragraph_paths(local as u32, flow(), &manifest, &mut backend, &|| false)
                    .unwrap();
                assert_eq!(paths.paragraph, flat + local as u32);
                if (row, column) != (2, 2) {
                    assert_eq!(
                        serde_json::to_value(&paths.computation).unwrap(),
                        serde_json::to_value(&reference_paths).unwrap()
                    );
                }
                outputs.push(serde_json::json!({"cell":text.source().cell,"source":text.source(),"result":result,"paths":paths}));
            }
        }
    }
    assert_eq!(outputs.len(), 10);
    // Missing declared family stays a source-bound diagnostic, with the table's
    // original flat paragraph index rather than the cell-local index zero.
    let other = read(&rewrite(&bytes, SLIDE, |s| {
        s.replace(FONT, "Unavailable Owned Family")
    }));
    let bad = prepared(&compiler(&other), 1, 2);
    let SourceTextError::FontSelection(error) = bad
        .shape_paragraph(0, &manifest, &mut backend, &|| false)
        .unwrap_err()
    else {
        panic!("font selection diagnostic")
    };
    assert_eq!(error.paragraph, 6);
    assert!(matches!(
        error.uses[0].font.declared_by.origin,
        TextStyleOrigin::TableStyle { .. }
    ));
    if let Some(dir) = std::env::var_os("MO_TABLE_TEXT_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("native-cell-text.pptx"), &bytes).unwrap();
        std::fs::write(
            dir.join("native-cell-text.json"),
            serde_json::to_vec_pretty(&outputs).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("missing-font.json"),
            serde_json::to_vec_pretty(&error).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn cell_preparation_diagnostics_and_resource_limits_use_the_real_scope() {
    let i = read(&fixture(true));
    let table = compiler(&i);
    assert!(matches!(
        table
            .prepare(
                SourceCellAddress { row: 1, column: 2 },
                Default::default(),
                &|| false
            )
            .unwrap(),
        SourceTextPreparation::Unresolved {
            issue: SourceTextIssue::CharacterProperty { paragraph: 6, .. }
        }
    ));
    for limits in [
        SourceTextLimits {
            max_plan_bytes: 0,
            ..Default::default()
        },
        SourceTextLimits {
            max_text_bytes: 0,
            ..Default::default()
        },
        SourceTextLimits {
            max_font_bindings: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            table.prepare(SourceCellAddress { row: 0, column: 2 }, limits, &|| false),
            Err(SourceTextError::Limit(_))
        ));
    }
    assert!(matches!(
        table.prepare(
            SourceCellAddress { row: 0, column: 2 },
            Default::default(),
            &|| true
        ),
        Err(SourceTextError::Cancelled)
    ));
}
