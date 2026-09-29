#[allow(dead_code)]
#[path = "../../../tools/test-support/source_table.rs"]
mod support;
use mo_geometry::{Fixed, Point, Rect};
use mo_pptx::{
    AuthorPlan,
    source::{table::SourceCellAddress, *},
};
use mo_presentation_compile::source_table::*;
use std::cell::Cell;
use support::*;
fn at(row: u32, column: u32) -> SourceCellAddress {
    SourceCellAddress { row, column }
}
fn prepared(i: &SourceIndex) -> DeclaredTableGeometry<'_> {
    DeclaredTableGeometry::prepare(i, &i.source_sha256, &target(i), Default::default(), &|| {
        false
    })
    .unwrap()
}
fn fixed(n: i64) -> Fixed {
    Fixed::from_raw((n as i128) << 32)
}
fn rect(left: i64, top: i64, right: i64, bottom: i64) -> Rect {
    Rect {
        min: Point {
            x: fixed(left),
            y: fixed(top),
        },
        max: Point {
            x: fixed(right),
            y: fixed(bottom),
        },
    }
}
#[test]
fn authored_and_reopened_table_share_merges_and_physical_text_ranges() {
    let (doc, defaults) = input();
    let authored = AuthorPlan::new(&doc, &defaults, Default::default(), &|| false).unwrap();
    let i = read(&bytes());
    let a = prepared(authored.declarations());
    let b = prepared(&i);
    assert_eq!(a.grid().regions().len(), 6);
    for row in 0..3 {
        for col in 0..3 {
            let at = at(row, col);
            let ac = a.cell(at).unwrap();
            let bc = b.cell(at).unwrap();
            assert_eq!(ac.region, bc.region);
            assert_eq!(ac.merged, bc.merged);
            assert_eq!(ac.physical, bc.physical);
            assert_eq!(ac.conversion_error_bound, Fixed::ZERO);
            assert_eq!(bc.conversion_error_bound, Fixed::ZERO);
            let text = b.body(at).unwrap().unwrap();
            assert_eq!(
                text.range,
                (row * 3 + col) as usize..(row * 3 + col + 1) as usize
            );
            assert_eq!(text.root.cell, Some(at));
            assert_eq!(a.body(at).unwrap().unwrap().paragraphs, text.paragraphs);
            assert!(std::ptr::eq(
                text.paragraphs,
                &i.surfaces[SLIDE].objects[0].paragraphs[text.range.clone()]
            ));
        }
    }
    let covered = b.cell(at(1, 1)).unwrap();
    assert_eq!(covered.region.origin, at(0, 0));
    assert_ne!(covered.physical, covered.merged);
    assert!(
        b.body(at(1, 1)).unwrap().unwrap().paragraphs[0][0]
            .text
            .contains("cell 1:1")
    );
    assert!(b.body(at(2, 2)).unwrap().unwrap().paragraphs[0].is_empty());
    assert!(b.cell(at(3, 0)).is_none());
}
#[test]
fn mixed_units_rtl_and_merges_use_grid_dimensions_not_a_resized_frame() {
    let b = rewrite(&bytes(), |s| {
        s.replacen("<a:tblPr>", "<a:tblPr rtl=\"1\">", 1)
            .replacen("<a:gridCol w=\"1828800\"/>", "<a:gridCol w=\"1in\"/>", 1)
            .replacen("<a:gridCol w=\"1828800\"/>", "<a:gridCol w=\"2.54cm\"/>", 1)
            .replacen("<a:gridCol w=\"1828800\"/>", "<a:gridCol w=\"72pt\"/>", 1)
    });
    let mut i = read(&b);
    let t = i.surfaces.get_mut(SLIDE).unwrap().objects[0]
        .table
        .as_mut()
        .unwrap();
    for (r, h) in t.rows.iter_mut().zip(["6pc", "6pi", "25.4mm"]) {
        r.height = h.to_owned().try_into().unwrap();
    }
    let g = prepared(&i);
    assert!(g.right_to_left());
    assert_eq!(
        g.columns().iter().map(|v| v.value).collect::<Vec<_>>(),
        [2743200, 1828800, 914400, 0].map(fixed)
    );
    assert_eq!(
        g.cell(at(1, 1)).unwrap().merged,
        rect(914400, 0, 2743200, 1828800)
    );
    assert_eq!(
        g.cell(at(0, 2)).unwrap().physical,
        rect(0, 0, 914400, 914400)
    );
    assert_eq!(
        g.cell(at(2, 2)).unwrap().physical,
        rect(0, 1828800, 914400, 2743200)
    );
}
#[test]
fn decimal_widths_are_accumulated_before_rounding_and_bounds_enclose_exact_edges() {
    let mut i = read(&bytes());
    let t = i.surfaces.get_mut(SLIDE).unwrap().objects[0]
        .table
        .as_mut()
        .unwrap();
    for c in &mut t.columns {
        c.width = "0.00000000000001pt".to_owned().try_into().unwrap();
    }
    for r in &mut t.rows {
        r.height = "0".to_owned().try_into().unwrap();
    }
    let g = prepared(&i);
    // Exact raw Q32 width is 127 * 2^32 / 10^12; each rounds to 1,
    // but all three together round to 2, rather than 3.
    assert_eq!(
        g.columns()
            .iter()
            .map(|b| b.value.raw())
            .collect::<Vec<_>>(),
        [0, 1, 1, 2]
    );
    for (n, b) in g.columns().iter().enumerate() {
        let exact_n = n as i128 * 127 * (1i128 << 32);
        let difference = (b.value.raw() * 1_000_000_000_000 - exact_n).abs();
        assert!(difference <= b.conversion_error_bound.raw() * 1_000_000_000_000);
    }
    assert_eq!(
        g.cell(at(0, 0)).unwrap().physical.min.y,
        g.cell(at(0, 0)).unwrap().physical.max.y
    );
    assert_eq!(g.rows().last().unwrap().conversion_error_bound, Fixed::ZERO);
}
#[test]
fn table_limits_digest_negative_dimensions_and_cancellation_are_explicit() {
    let i = read(&bytes());
    let limits = TableGeometryLimits::default();
    for limits in [
        TableGeometryLimits {
            max_objects: 0,
            ..limits
        },
        TableGeometryLimits {
            max_coordinate_bytes: 0,
            ..limits
        },
    ] {
        assert!(matches!(
            DeclaredTableGeometry::prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
            Err(TableGeometryError::Limit(_))
        ));
    }
    let other = mo_common::Digest::from_sha256([0xaa; 32]);
    assert!(matches!(
        DeclaredTableGeometry::prepare(&i, &other, &target(&i), limits, &|| false),
        Err(TableGeometryError::Source(_))
    ));
    let calls = Cell::new(0);
    assert!(matches!(
        DeclaredTableGeometry::prepare(&i, &i.source_sha256, &target(&i), limits, &|| {
            calls.set(calls.get() + 1);
            calls.get() == 35
        }),
        Err(TableGeometryError::Cancelled)
    ));
    for lexical in ["-1", "-0.0000000000000000000000000000000000000001pt"] {
        let mut i = i.clone();
        let t = i.surfaces.get_mut(SLIDE).unwrap().objects[0]
            .table
            .as_mut()
            .unwrap();
        t.columns[0].width = lexical.to_owned().try_into().unwrap();
        assert!(matches!(
            DeclaredTableGeometry::prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
            Err(TableGeometryError::NegativeDimension { .. })
        ));
    }
    let mut i = i;
    i.surfaces.get_mut(SLIDE).unwrap().objects[0]
        .table
        .as_mut()
        .unwrap()
        .columns[0]
        .width = format!("1.{}pt", "0".repeat(257)).try_into().unwrap();
    assert!(matches!(
        DeclaredTableGeometry::prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
        Err(TableGeometryError::Limit(_))
    ));
}
#[test]
fn body_bindings_reject_other_cell_roots_overlaps_and_unrelated_owners() {
    let base = read(&bytes());
    for variant in 0..4 {
        let mut i = base.clone();
        let surface = i.surfaces.get_mut(SLIDE).unwrap();
        let object = &mut surface.objects[0];
        let t = object.table.as_mut().unwrap();
        match variant {
            0 => t.rows[0].cells[1].text_body_ordinal = t.rows[0].cells[0].text_body_ordinal,
            1 => t.rows[0].cells[1].paragraph_start = 0,
            2 => t.rows[0].cells[1].paragraph_count = u32::MAX,
            _ => {
                let root = surface
                    .text
                    .roots
                    .iter_mut()
                    .find(|r| r.cell == Some(at(0, 1)))
                    .unwrap();
                root.owner = Some(object.native_id + 1);
            }
        }
        assert!(matches!(
            DeclaredTableGeometry::prepare(
                &i,
                &i.source_sha256,
                &target(&i),
                Default::default(),
                &|| false
            ),
            Err(TableGeometryError::Source(_))
        ));
    }
}

#[test]
fn thousands_of_physical_roots_bind_once_and_keep_flat_addresses() {
    use std::fmt::Write;
    let n = 64u32;
    let b = rewrite(&bytes(), |s| {
        let start = s.find("<a:tbl>").unwrap();
        let end = s.find("</a:tbl>").unwrap() + "</a:tbl>".len();
        let mut table = String::from("<a:tbl><a:tblPr/><a:tblGrid>");
        for _ in 0..n {
            table.push_str("<a:gridCol w=\"1\"/>");
        }
        table.push_str("</a:tblGrid>");
        for row in 0..n {
            table.push_str("<a:tr h=\"1\">");
            for col in 0..n {
                write!(table, "<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>cell {row}:{col}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>").unwrap();
            }
            table.push_str("</a:tr>");
        }
        table.push_str("</a:tbl>");
        format!("{}{}{}", &s[..start], table, &s[end..])
    });
    let i = read(&b);
    let g = prepared(&i);
    assert_eq!(g.grid().regions().len(), (n * n) as usize);
    for row in 0..n {
        for col in 0..n {
            let c = at(row, col);
            let binding = g.body(c).unwrap().unwrap();
            assert_eq!(binding.range.start, (row * n + col) as usize);
            assert_eq!(binding.root.cell, Some(c));
            assert_eq!(binding.paragraphs[0][0].text, format!("cell {row}:{col}"));
            let geometry = g.cell(c).unwrap();
            assert_eq!(
                geometry.physical,
                rect(col as i64, row as i64, (col + 1) as i64, (row + 1) as i64)
            );
        }
    }
    let mut limits = TableGeometryLimits::default();
    limits.grid.max_cells = (n * n - 1) as usize;
    assert!(matches!(
        DeclaredTableGeometry::prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
        Err(TableGeometryError::Grid(_))
    ));
}

#[test]
fn missing_text_body_and_zero_dimensions_remain_distinct_from_empty_paragraphs() {
    let b = rewrite(&bytes(), |s| {
        let start = s.find("<a:txBody>").unwrap();
        let end = s[start..].find("</a:txBody>").unwrap() + start + "</a:txBody>".len();
        format!("{}{}", &s[..start], &s[end..])
    });
    let i = read(&b);
    let g = prepared(&i);
    assert!(g.body(at(0, 0)).unwrap().is_none());
    assert_eq!(g.body(at(0, 1)).unwrap().unwrap().range, 0..1);
    assert!(g.body(at(2, 2)).unwrap().unwrap().paragraphs[0].is_empty());
    assert_eq!(g.grid().regions().len(), 6);
    let mut i = i;
    let object = &mut i.surfaces.get_mut(SLIDE).unwrap().objects[0];
    object.table.as_mut().unwrap().rows[0].cells[0].paragraph_start = u32::MAX;
    assert!(
        mo_pptx::source::text::bind_body(
            object,
            &mo_pptx::source::text::SourceTextCatalog::default(),
            Some(at(0, 0))
        )
        .is_err()
    );
}
