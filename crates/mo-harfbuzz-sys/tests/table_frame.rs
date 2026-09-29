//! Declared table grids and real native glyph layout, including merged cells.
#[allow(dead_code)]
#[path = "../../../tools/test-support/table_text.rs"]
mod support;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_pptx::source::{SourceIndex, table::SourceCellAddress};
use mo_presentation_compile::{
    source_frame::{capacity::*, *},
    source_table::*,
};
use mo_text::manifest::*;
use support::*;

fn emu(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
fn cell(row: u32, column: u32) -> SourceCellAddress {
    SourceCellAddress { row, column }
}
fn bound(i: &SourceIndex) -> TableFrameCompiler<'_> {
    TableFrameCompiler::bind(i, &i.source_sha256, &target(i), Default::default(), &|| {
        false
    })
    .unwrap()
}
fn frame(table: &TableFrameCompiler<'_>, cell: SourceCellAddress) -> SourceFramePlan {
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
    table
        .compile(
            cell,
            &manifest,
            &mut NativeShaper::default(),
            Default::default(),
            Fixed::from_raw(1 << 26),
            &|| false,
        )
        .unwrap()
}
fn evidence(name: &str, bytes: &[u8], frames: &[SourceFramePlan]) {
    if let Some(dir) = std::env::var_os("MO_TABLE_FRAME_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pptx")), bytes).unwrap();
        std::fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(frames).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn merged_cell_uses_the_grid_extent_and_real_cell_margins() {
    let bytes = fixture(false);
    let i = read(&bytes);
    let table = bound(&i);
    let shared = table.geometry().shared_grid();
    assert!(std::sync::Arc::strong_count(&shared) >= 3); // geometry, text, this test
    let merged = frame(&table, cell(0, 0));
    assert_eq!(merged.text.cell, Some(cell(0, 0)));
    assert_eq!(merged.region.outer.min.x, Fixed::ZERO);
    assert_eq!(merged.region.outer.max.x, emu(3_657_600));
    assert_eq!(merged.region.outer.max.y, emu(1_828_800));
    assert_eq!(merged.region.inner.min.x, emu(10_000));
    assert_eq!(merged.region.inner.max.x, emu(3_627_600));
    assert_eq!(merged.region.inner.min.y, emu(20_000));
    assert_eq!(merged.region.inner.max.y, emu(1_788_800));
    assert!(merged.clip.is_none()); // fixture explicitly requests overflow
    assert_eq!(merged.work.glyphs as usize, merged.glyphs.len());
    assert!(!merged.glyphs.is_empty());
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
    for covered in [cell(0, 1), cell(1, 0), cell(1, 1)] {
        assert!(
            matches!(table.compile(covered, &manifest, &mut NativeShaper::default(), Default::default(), Fixed::from_raw(1 << 26), &|| false), Err(SourceFrameError::Mapping(issue)) if matches!(*issue, SourceFrameIssue::CoveredCell {cell: c,origin} if c == covered && origin == cell(0,0)))
        );
    }
    let single = frame(&table, cell(1, 2));
    assert_eq!(single.region.outer.min.x, emu(3_657_600));
    assert_eq!(single.region.outer.min.y, emu(914_400));
    assert_eq!(single.text.paragraph_start, 6);
    assert_eq!(single.paragraphs.len(), 1);
    let capacity = TextCapacity {
        profile: capacity::PROFILE.into(),
        frames: vec![
            measure(&merged, &|| false).unwrap(),
            measure(&single, &|| false).unwrap(),
        ],
    };
    capacity.validate(2, &|| false).unwrap();
    let mut duplicate = capacity.clone();
    duplicate.frames[1] = duplicate.frames[0].clone();
    assert!(duplicate.validate(2, &|| false).is_err());
    evidence("merged", &bytes, &[merged, single]);
}

#[test]
fn cell_anchor_rtl_grid_and_overflow_remain_separate_from_paint_clipping() {
    for rtl in [false, true] {
        let mut outputs = vec![];
        let mut fixtures = vec![];
        for anchor in ["t", "ctr", "b"] {
            let bytes = rewrite(&fixture(false), SLIDE, |s| {
                s.replace("<a:tblPr", &format!("<a:tblPr rtl=\"{}\"", u8::from(rtl)))
                    .replace("anchor=\"ctr\"", &format!("anchor=\"{anchor}\""))
                    .replace("horzOverflow=\"overflow\"", "horzOverflow=\"clip\"")
            });
            let i = read(&bytes);
            let r = frame(&bound(&i), cell(0, 0));
            let clip = r.clip.unwrap();
            assert!(clip.horizontal && !clip.vertical);
            assert_eq!(clip.bounds, r.region.outer);
            assert_eq!(r.region.outer.min.x, emu(if rtl { 1_828_800 } else { 0 }));
            assert_eq!(
                r.region.inner.min.x,
                emu(if rtl { 1_838_800 } else { 10_000 })
            );
            fixtures.push(bytes);
            outputs.push(r);
        }
        let free = outputs[0]
            .region
            .inner
            .max
            .y
            .checked_sub(outputs[0].region.inner.min.y)
            .unwrap()
            .checked_sub(outputs[0].content_height)
            .unwrap();
        for (index, shift) in [(1, free.half().unwrap()), (2, free)] {
            assert_eq!(
                outputs[index].glyphs[0]
                    .origin
                    .y
                    .checked_sub(outputs[0].glyphs[0].origin.y)
                    .unwrap(),
                shift
            );
        }
        for (n, (bytes, r)) in fixtures.iter().zip(&outputs).enumerate() {
            evidence(
                &format!("rtl-{rtl}-anchor-{n}"),
                bytes,
                std::slice::from_ref(r),
            );
        }
    }
    let bytes = rewrite(&fixture(false), SLIDE, |s| {
        s.replace("A A", &"A".repeat(100))
            .replace("horzOverflow=\"overflow\"", "horzOverflow=\"clip\"")
    });
    let i = read(&bytes);
    let r = frame(&bound(&i), cell(0, 2));
    let c = measure(&r, &|| false).unwrap();
    assert_eq!(r.work.glyphs, 100);
    assert!(c.horizontal_overflow_lines > 0 || c.vertical_excess > Fixed::ZERO);
    assert_eq!(c.cell, Some(cell(0, 2)));
    assert_eq!(c.ink_bounds, r.bounds);
    evidence("overflow", &bytes, &[r]);
}

#[test]
fn cell_frame_diagnostics_keep_flat_paragraph_and_fail_before_component_work() {
    let bytes = rewrite(&fixture(false), SLIDE, |s| {
        s.replace("<a:p>", "<a:p><a:pPr fontAlgn=\"t\"/>")
    });
    let i = read(&bytes);
    let table = bound(&i);
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
    assert!(
        matches!(table.compile(cell(1,2),&manifest,&mut NativeShaper::default(),Default::default(),Fixed::from_raw(1 << 26),&||false),Err(SourceFrameError::Mapping(issue)) if matches!(*issue,SourceFrameIssue::ParagraphProperty {paragraph:6,..}))
    );
    assert!(
        table
            .compile(
                cell(1, 2),
                &manifest,
                &mut NativeShaper::default(),
                SourceFrameLimits {
                    max_paragraphs: 0,
                    ..Default::default()
                },
                Fixed::from_raw(1 << 26),
                &|| false
            )
            .is_err()
    );
    assert!(
        table
            .compile(
                cell(1, 2),
                &manifest,
                &mut NativeShaper::default(),
                Default::default(),
                Fixed::from_raw(1 << 26),
                &|| true
            )
            .is_err()
    );
}
