//! Prepared and independent query paths must retain the same source semantics.
use super::page::{page_request, uniform_borders};
use super::*;
use mo_presentation_source::source::{
    prepared::*,
    table::{TableCellEdge, borders::*},
};
use std::sync::Arc;
fn reference(id: u32) -> SourceObjectRef {
    SourceObjectRef {
        part: SLIDE.into(),
        native_id: id,
    }
}
fn ready<'a>(context: &mut SourcePreparation<'a>, id: u32) -> Arc<PreparedSourceTable<'a>> {
    match context.table(&reference(id), &|| false).unwrap() {
        SourceTablePreparation::Prepared { table } => table,
        SourceTablePreparation::InvalidGrid { .. } => panic!("owned rectangular table"),
    }
}
#[test]
fn one_grid_and_style_are_shared_by_fill_border_and_text_consumers() {
    let index = read(&uniform_borders(&fixture(false)));
    let mut context =
        SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
            .unwrap();
    let table = ready(&mut context, 2);
    let mut targets = vec![FillTarget::TableBackground { native_id: 2 }];
    targets.extend(
        table
            .grid()
            .regions()
            .iter()
            .map(|r| FillTarget::TableCell {
                native_id: 2,
                cell: r.origin,
            }),
    );
    let q = SourceFillColorQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        targets,
        fill_profile: FillProfile::Drawingml2024DraftV1,
        color_profile: mo_presentation_source::source::color::ColorProfile::Ecma3762016DraftV1,
        context: Default::default(),
    };
    let old = mo_presentation_source::source::fill::colors::query(
        &index,
        &q,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let shared = mo_presentation_source::source::fill::colors::query_in_preparation(
        &mut context,
        &q,
        SLIDE,
        SLIDE,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(old).unwrap(),
        serde_json::to_value(shared).unwrap()
    );
    let targets = (0..3)
        .flat_map(|row| {
            (0..3).flat_map(move |column| {
                [
                    TableCellEdge::Left,
                    TableCellEdge::Right,
                    TableCellEdge::Top,
                    TableCellEdge::Bottom,
                    TableCellEdge::TopLeftToBottomRight,
                    TableCellEdge::BottomLeftToTopRight,
                ]
                .map(|edge| TableBorderTarget {
                    native_id: 2,
                    cell: SourceCellAddress { row, column },
                    edge,
                })
            })
        })
        .collect();
    let q = SourceTableBorderQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        targets,
        line_profile:
            mo_presentation_source::source::line::resolve::LineProfile::Drawingml2024DraftV1,
        fill_profile: q.fill_profile,
        color_profile: q.color_profile,
        context: q.context,
    };
    let old = mo_presentation_source::source::table::borders::query(
        &index,
        &q,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let shared = mo_presentation_source::source::table::borders::query_in_preparation(
        &mut context,
        &q,
        SLIDE,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(old).unwrap(),
        serde_json::to_value(shared).unwrap()
    );
    let again = ready(&mut context, 2);
    assert!(Arc::ptr_eq(&table, &again));
    assert!(Arc::ptr_eq(&table.shared_grid(), &again.shared_grid()));
    assert!(Arc::ptr_eq(
        &table.style().unwrap(),
        &again.style().unwrap()
    ));
    let work = context.work();
    assert_eq!(work.compiled_tables, 1);
    assert_eq!(work.grid_cells, 9);
    assert_eq!(work.grid_steps, 21);
    assert_eq!(
        work.indexed_objects,
        index
            .surfaces
            .values()
            .map(|s| s.objects.len())
            .sum::<usize>()
    );
    let prepared = crate::source_page::preflight_sampled(
        &index,
        &page_request(&index),
        true,
        true,
        None,
        &|| false,
    )
    .unwrap();
    let layout = &prepared.tables[&(SLIDE.into(), 2)];
    assert!(Arc::ptr_eq(
        &layout.source.shared_grid(),
        &layout.geometry.shared_grid()
    ));
    let compiler =
        crate::source_table::TableFrameCompiler::bind_prepared(layout, &|| false).unwrap();
    assert!(Arc::ptr_eq(&layout.geometry, &compiler.geometry));
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let plain = crate::source_table::TableFrameCompiler::bind(
        &index,
        &index.source_sha256,
        &reference(2),
        Default::default(),
        &|| false,
    )
    .unwrap();
    for region in table.grid().regions() {
        let old = plain
            .compile(
                region.origin,
                &manifest,
                &mut NativeShaper::default(),
                Default::default(),
                Fixed::from_raw(1 << 24),
                &|| false,
            )
            .unwrap();
        let shared = compiler
            .compile(
                region.origin,
                &manifest,
                &mut NativeShaper::default(),
                Default::default(),
                Fixed::from_raw(1 << 24),
                &|| false,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(old).unwrap(),
            serde_json::to_value(shared).unwrap()
        );
    }
}

#[test]
fn source_preparation_limits_invalid_grids_and_cancellation_cannot_reset_work() {
    let index = read(&super::cases::two_tables(&uniform_borders(&fixture(false))));
    let bad = mo_common::Digest::from_sha256([0; 32]);
    assert!(SourcePreparation::new(&index, &bad, Default::default(), &|| false).is_err());
    for limits in [
        SourcePreparationLimits {
            max_objects: 0,
            ..Default::default()
        },
        SourcePreparationLimits {
            max_surfaces: 0,
            ..Default::default()
        },
    ] {
        assert!(SourcePreparation::new(&index, &index.source_sha256, limits, &|| false).is_err());
    }
    let limits = SourcePreparationLimits {
        grids: mo_presentation_source::source::table::grid::NativeTableGridLimits {
            max_cells: 17,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut context =
        SourcePreparation::new(&index, &index.source_sha256, limits, &|| false).unwrap();
    ready(&mut context, 2);
    assert!(context.table(&reference(990), &|| false).is_err());
    assert_eq!(context.work().compiled_tables, 1);
    assert!(context.table(&reference(2), &|| false).is_err());
    let mut malformed = index.clone();
    malformed
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
    let mut context =
        SourcePreparation::new(&malformed, &malformed.source_sha256, limits, &|| false).unwrap();
    for _ in 0..2 {
        assert!(matches!(
            context.table(&reference(2), &|| false).unwrap(),
            SourceTablePreparation::InvalidGrid { .. }
        ));
    }
    assert_eq!(context.work().compiled_tables, 1);
    assert_eq!(context.work().grid_cells, 9);
    assert!(context.table(&reference(990), &|| false).is_err());
    let mut context =
        SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
            .unwrap();
    ready(&mut context, 2);
    assert!(context.table(&reference(2), &|| true).is_err());
    assert!(context.table(&reference(2), &|| false).is_err());
    let background = SourceFillQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        targets: vec![FillTarget::Background {}],
        profile: FillProfile::Drawingml2024DraftV1,
    };
    assert!(
        mo_presentation_source::source::fill::resolve::query_in_preparation(
            &mut context,
            &background,
            SLIDE,
            SLIDE,
            Default::default(),
            &|| false
        )
        .is_err()
    );
    let mut duplicate = index.clone();
    let object = duplicate.surfaces[SLIDE].objects[0].clone();
    duplicate
        .surfaces
        .get_mut(SLIDE)
        .unwrap()
        .objects
        .push(object);
    assert!(
        SourcePreparation::new(
            &duplicate,
            &duplicate.source_sha256,
            Default::default(),
            &|| false
        )
        .is_err()
    );
}

#[test]
fn shared_queries_keep_failure_semantics_and_logical_admission() {
    use mo_presentation_source::source::text::cascade::{TableTextBinding, TableTextResolver};
    let base = read(&uniform_borders(&fixture(false)));
    for broken in [false, true] {
        let mut index = base.clone();
        if broken {
            let table = index
                .surfaces
                .get_mut(SLIDE)
                .unwrap()
                .objects
                .iter_mut()
                .find(|o| o.native_id == 2)
                .unwrap()
                .table
                .as_mut()
                .unwrap();
            let properties = table.properties.as_mut().unwrap();
            properties.inline_style = None;
            properties.style_id = Some("{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}".into());
            index.table_styles = None;
        }
        let q = SourceFillQuery {
            expected_source_sha256: index.source_sha256.clone(),
            surface: SLIDE.into(),
            targets: vec![FillTarget::TableCell {
                native_id: 2,
                cell: SourceCellAddress { row: 0, column: 0 },
            }],
            profile: FillProfile::Drawingml2024DraftV1,
        };
        let mut context =
            SourcePreparation::new(&index, &index.source_sha256, Default::default(), &|| false)
                .unwrap();
        let limits = FillResolveLimits {
            max_values: 0,
            ..Default::default()
        };
        assert!(
            mo_presentation_source::source::fill::resolve::query_in_preparation(
                &mut context,
                &q,
                SLIDE,
                SLIDE,
                limits,
                &|| false
            )
            .is_err()
        );
        assert_eq!(context.work().compiled_tables, 0);
        let old = mo_presentation_source::source::fill::resolve::query(
            &index,
            &q,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let shared = mo_presentation_source::source::fill::resolve::query_in_preparation(
            &mut context,
            &q,
            SLIDE,
            SLIDE,
            Default::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(old).unwrap(),
            serde_json::to_value(shared).unwrap()
        );
        assert!(
            mo_presentation_source::source::fill::resolve::query_in_preparation(
                &mut context,
                &q,
                SLIDE,
                SLIDE,
                limits,
                &|| false
            )
            .is_err()
        );
        assert_eq!(context.work().compiled_tables, 1);
        let mut wrong = q.clone();
        wrong.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
        assert!(
            mo_presentation_source::source::fill::resolve::query_in_preparation(
                &mut context,
                &wrong,
                SLIDE,
                SLIDE,
                Default::default(),
                &|| false
            )
            .is_err()
        );
        if broken {
            let prepared = ready(&mut context, 2);
            let old = TableTextResolver::bind(
                &index,
                &index.source_sha256,
                &reference(2),
                Default::default(),
                Default::default(),
                &|| false,
            )
            .unwrap();
            let shared =
                TableTextResolver::bind_prepared(&prepared, Default::default(), &|| false).unwrap();
            let (
                TableTextBinding::Unresolved { reason: old },
                TableTextBinding::Unresolved { reason: shared },
            ) = (old, shared)
            else {
                panic!("missing style remains unresolved")
            };
            assert_eq!(
                serde_json::to_value(old).unwrap(),
                serde_json::to_value(shared).unwrap()
            );
        }
    }
}

#[test]
fn shared_page_preparation_retains_text_budgets_and_source_identity() {
    let index = read(&uniform_borders(&fixture(false)));
    let q = page_request(&index);
    let prepared =
        crate::source_page::preflight_sampled(&index, &q, true, true, None, &|| false).unwrap();
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    for wrong_source in [false, true] {
        let mut shaper = Shaper::default();
        let mut limits = TextPageLimits::default();
        if !wrong_source {
            limits.tables.grid.max_cells = 8;
        }
        let mut c = Compiler::new(&manifest, &mut shaper, limits);
        let copied_index = index.clone();
        let source = if wrong_source { &copied_index } else { &index };
        assert!(
            c.preflight_shared(
                source,
                &q,
                prepared.objects.iter().map(|o| &o.binding),
                &prepared.tables,
                &|| false
            )
            .is_err()
        );
        assert!(c.finish().is_err());
        assert_eq!(shaper.calls, 0);
    }
    let mut shaper = Shaper::default();
    let mut c = Compiler::new(&manifest, &mut shaper, Default::default());
    let mut wrong = q.clone();
    wrong.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(
        c.preflight_shared(
            &index,
            &wrong,
            prepared.objects.iter().map(|o| &o.binding),
            &prepared.tables,
            &|| false
        )
        .is_err()
    );
    assert!(c.finish().is_err());
    assert_eq!(shaper.calls, 0);
}
