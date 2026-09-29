//! Owned retained preparation: source lifetime, stable topology, exact queries
//! and sampled placement must agree with independently prepared native pages.
use super::page::{page_request, uniform_borders};
use super::*;
use mo_presentation_source::source::prepared::*;
use std::sync::Arc;
#[allow(dead_code)]
#[path = "../../../../../tools/test-support/table_styles.rs"]
mod styles;
fn reference(id: u32) -> SourceObjectRef {
    SourceObjectRef {
        part: SLIDE.into(),
        native_id: id,
    }
}
fn ready<'a>(context: &mut SourcePreparation<'a>, id: u32) -> Arc<PreparedSourceTable<'a>> {
    match context.table(&reference(id), &|| false).unwrap() {
        SourceTablePreparation::Prepared { table } => table,
        _ => panic!("owned valid grid"),
    }
}
#[test]
fn retained_source_owns_topology_and_rejects_uncaptured_objects_without_poisoning_owner() {
    let index = Arc::new(read(&super::cases::two_tables(&uniform_borders(&fixture(
        false,
    )))));
    let mut context =
        SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
            .unwrap();
    let grid = ready(&mut context, 2);
    let regions = grid.grid().regions().as_ptr();
    drop(grid);
    let owner = context.retain(Arc::clone(&index), &|| false).unwrap();
    drop(index);
    assert_eq!(owner.work().compiled_tables, 1);
    for _ in 0..3 {
        let mut session = owner.session(&|| false).unwrap();
        let table = ready(&mut session, 2);
        assert_eq!(table.grid().regions().as_ptr(), regions);
        assert_eq!(session.work().compiled_tables, 0);
        assert_eq!(session.work().grid_cells, 0);
        assert_eq!(session.work().reused_tables, 1);
        assert!(session.table(&reference(990), &|| false).is_err());
        assert!(session.table(&reference(2), &|| false).is_err());
    }
    let mut session = owner.session(&|| false).unwrap();
    assert!(session.table(&reference(2), &|| true).is_err());
    assert!(session.table(&reference(2), &|| false).is_err());
    assert!(owner.session(&|| true).is_err());
    ready(&mut owner.session(&|| false).unwrap(), 2);
    let copied = Arc::new(owner.index().clone());
    assert!(
        owner
            .session(&|| false)
            .unwrap()
            .retain(copied, &|| false)
            .is_err()
    );
}
#[test]
fn retained_table_styles_keep_shared_inline_absent_and_missing_definitions() {
    let base = uniform_borders(&fixture(false));
    let absent = rewrite(&base, SLIDE, |s| {
        let start = s.find("<a:tableStyle ").unwrap();
        let end = start + s[start..].find("</a:tableStyle>").unwrap() + "</a:tableStyle>".len();
        format!("{}{}", &s[..start], &s[end..])
    });
    let shared = styles::catalog(
        &styles::inline(
            &absent,
            &format!("<a:tableStyleId>{}</a:tableStyleId>", styles::ID),
        ),
        &styles::list(&styles::full_style("tblStyle", styles::ID)),
    );
    let missing = styles::inline(
        &absent,
        &format!("<a:tableStyleId>{}</a:tableStyleId>", styles::ID),
    );
    for bytes in [base, absent, shared, missing] {
        let index = Arc::new(read(&bytes));
        let q = SourceFillColorQuery {
            expected_source_sha256: index.source_sha256.clone(),
            surface: SLIDE.into(),
            targets: vec![
                FillTarget::TableBackground { native_id: 2 },
                FillTarget::TableCell {
                    native_id: 2,
                    cell: SourceCellAddress { row: 0, column: 0 },
                },
            ],
            fill_profile: FillProfile::Drawingml2024DraftV1,
            color_profile: mo_presentation_source::source::color::ColorProfile::Ecma3762016DraftV1,
            context: Default::default(),
        };
        let mut context =
            SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
                .unwrap();
        ready(&mut context, 2);
        let owner = context.retain(Arc::clone(&index), &|| false).unwrap();
        drop(index);
        let expected = mo_presentation_source::source::fill::colors::query(
            owner.index(),
            &q,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let actual = mo_presentation_source::source::fill::colors::query_in_preparation(
            &mut owner.session(&|| false).unwrap(),
            &q,
            SLIDE,
            SLIDE,
            Default::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(expected).unwrap(),
            serde_json::to_value(actual).unwrap()
        );
    }
}
#[test]
fn retained_page_shares_coordinates_and_grid_but_resamples_world_geometry() {
    let bytes = super::page::gradient_cell(&uniform_borders(&fixture(false)), "circle");
    let bytes = rewrite(&bytes, SLIDE, |s| {
        s.replace("<a:tblPr", "<a:tblPr rtl=\"1\"")
    });
    let index = Arc::new(read(&super::cases::two_tables(&bytes)));
    let q = page_request(&index);
    let mut initial =
        crate::source_page::preflight_sampled(&index, &q, true, true, None, &|| false).unwrap();
    let grids: Vec<_> = initial
        .tables
        .iter()
        .map(|(key, t)| {
            (
                key.clone(),
                t.source.grid().regions().as_ptr(),
                t.geometry.retained_layout(),
            )
        })
        .collect();
    let retained = crate::source_table::RetainedTables::capture(
        Arc::clone(&index),
        initial.source.take().unwrap(),
        &initial.tables,
        &|| false,
    )
    .unwrap();
    drop(initial);
    assert_eq!(retained.source.work().compiled_tables, 2);
    for step in 0..5 {
        let transforms = std::collections::BTreeMap::from([
            (
                (SLIDE.into(), 2),
                crate::sampled_properties::SampledProperties {
                    motion: Some([
                        crate::interval::Interval::ratio(step, 100),
                        crate::interval::Interval::ratio(-step, 100),
                    ]),
                    ..Default::default()
                },
            ),
            (
                (SLIDE.into(), 990),
                crate::sampled_properties::SampledProperties {
                    opacity: Some(30000),
                    visibility: Some(if step == 2 {
                        mo_timeline::Visibility::Hidden
                    } else {
                        mo_timeline::Visibility::Visible
                    }),
                    ..Default::default()
                },
            ),
        ]);
        let cached = crate::source_page::preflight_retained(
            &index,
            &q,
            true,
            true,
            Some(&transforms),
            Some(&retained),
            &|| false,
        )
        .unwrap();
        let work = cached.source.as_ref().unwrap().work();
        assert_eq!(work.compiled_tables, 0);
        assert_eq!(work.reused_tables, if step == 2 { 1 } else { 2 });
        for (key, pointer, layout) in &grids {
            if let Some(t) = cached.tables.get(key) {
                assert_eq!(t.source.grid().regions().as_ptr(), *pointer);
                assert!(Arc::ptr_eq(&t.geometry.retained_layout(), layout));
            }
        }
        let fresh = crate::source_page::preflight_sampled(
            &index,
            &q,
            true,
            true,
            Some(&transforms),
            &|| false,
        )
        .unwrap();
        let cached =
            crate::source_page::build(cached, None, &Default::default(), &|| false).unwrap();
        let fresh = crate::source_page::build(fresh, None, &Default::default(), &|| false).unwrap();
        assert_eq!(
            serde_json::to_value(&cached.raster).unwrap(),
            serde_json::to_value(&fresh.raster).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&cached.bindings).unwrap(),
            serde_json::to_value(&fresh.bindings).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&cached.info).unwrap(),
            serde_json::to_value(&fresh.info).unwrap()
        );
    }
    let mut mismatched =
        crate::source_page::preflight_sampled(&index, &q, true, true, None, &|| false).unwrap();
    let wrong_geometry = Arc::clone(&mismatched.tables[&(SLIDE.into(), 990)].geometry);
    mismatched
        .tables
        .get_mut(&(SLIDE.into(), 2))
        .unwrap()
        .geometry = wrong_geometry;
    assert!(
        crate::source_table::RetainedTables::capture(
            Arc::clone(&index),
            mismatched.source.take().unwrap(),
            &mismatched.tables,
            &|| false
        )
        .is_err()
    );
    let copied = index.as_ref().clone();
    assert!(
        crate::source_page::preflight_retained(
            &copied,
            &q,
            true,
            true,
            None,
            Some(&retained),
            &|| false
        )
        .is_err()
    );
    assert!(
        crate::source_page::preflight_retained(
            &index,
            &q,
            true,
            true,
            None,
            Some(&retained),
            &|| true
        )
        .is_err()
    );
    crate::source_page::preflight_retained(&index, &q, true, true, None, Some(&retained), &|| {
        false
    })
    .unwrap();
    let mut budget = crate::source_table::TablePreparationBudget::new(
        crate::source_table::TableGeometryLimits {
            grid: mo_presentation_source::source::table::grid::NativeTableGridLimits {
                max_cells: 17,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    retained
        .admit(&reference(2), &mut budget, &|| false)
        .unwrap();
    assert!(
        retained
            .admit(&reference(990), &mut budget, &|| false)
            .is_err()
    );
}

#[test]
fn retained_invalid_grid_and_cancelled_capture_keep_admission_and_atomicity() {
    let mut index = read(&uniform_borders(&fixture(false)));
    index
        .surfaces
        .get_mut(SLIDE)
        .unwrap()
        .objects
        .iter_mut()
        .find(|o| o.native_id == 2)
        .unwrap()
        .table
        .as_mut()
        .unwrap()
        .rows[0]
        .cells
        .pop();
    let index = Arc::new(index);
    let mut context =
        SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
            .unwrap();
    let SourceTablePreparation::InvalidGrid {
        reason: expected,
        cost,
    } = context.table(&reference(2), &|| false).unwrap()
    else {
        panic!("invalid grid")
    };
    let owner = context.retain(Arc::clone(&index), &|| false).unwrap();
    for _ in 0..3 {
        let mut session = owner.session(&|| false).unwrap();
        let SourceTablePreparation::InvalidGrid {
            reason,
            cost: actual,
        } = session.table(&reference(2), &|| false).unwrap()
        else {
            panic!("cached invalid grid")
        };
        assert_eq!(reason, expected);
        assert_eq!(actual.cells, cost.cells);
        assert_eq!(session.work().compiled_tables, 0);
        assert_eq!(session.work().reused_tables, 1);
        let frozen = session.retain(Arc::clone(&index), &|| false).unwrap();
        assert_eq!(frozen.work().grid_cells, 9);
        assert_eq!(frozen.work().compiled_tables, 1);
    }
    let mut context =
        SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
            .unwrap();
    context.table(&reference(2), &|| false).unwrap();
    let calls = Cell::new(0);
    let check = || {
        let n = calls.get() + 1;
        calls.set(n);
        n == 3
    };
    assert!(context.retain(Arc::clone(&index), &check).is_err());
    assert_eq!(calls.get(), 3);
    let mut failed = owner.session(&|| false).unwrap();
    assert!(failed.table(&reference(2), &|| true).is_err());
    assert!(failed.retain(Arc::clone(&index), &|| false).is_err());
}
