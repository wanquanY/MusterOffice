//! Standalone image queries bind real table receivers once per native object.
//! Page composition already has these prepared regions and uses layout_region.
use super::ImageSourceLayoutError as E;
use crate::source_table::{DeclaredTableGeometry, TableGeometryError, TablePreparationBudget};
use mo_geometry::{Fixed, Rect};
use mo_presentation_source::source::{
    SourceIndex, SourceObjectRef,
    fill::resolve::FillTarget,
    images::SourceImageResources,
    prepared::{SourcePreparation, SourceTablePreparation},
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn prepare(
    index: &SourceIndex,
    catalog: &SourceImageResources,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<FillTarget, (Rect, Fixed)>, E> {
    let targets: BTreeSet<_> = catalog
        .targets
        .iter()
        .filter_map(|v| match v.target {
            FillTarget::TableCell { .. } | FillTarget::TableBackground { .. } => {
                Some(v.target.clone())
            }
            _ => None,
        })
        .collect();
    if targets.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut source =
        SourcePreparation::new(index, &catalog.source_sha256, Default::default(), check)
            .map_err(TableGeometryError::from)?;
    let mut budget = TablePreparationBudget::new(Default::default());
    let mut tables = BTreeMap::new();
    let mut output = BTreeMap::new();
    for target in targets {
        super::cancel(check)?;
        let id = match target {
            FillTarget::TableCell { native_id, .. } | FillTarget::TableBackground { native_id } => {
                native_id
            }
            _ => unreachable!("table receiver targets"),
        };
        if let std::collections::btree_map::Entry::Vacant(entry) = tables.entry(id) {
            let prepared = source
                .table(
                    &SourceObjectRef {
                        part: catalog.surface.clone(),
                        native_id: id,
                    },
                    check,
                )
                .map_err(TableGeometryError::from)?;
            let SourceTablePreparation::Prepared { table } = prepared else {
                let SourceTablePreparation::InvalidGrid { reason, .. } = prepared else {
                    unreachable!()
                };
                return Err(TableGeometryError::Grid(
                    mo_presentation_source::source::table::grid::NativeTableGridError::Invalid(
                        reason,
                    ),
                )
                .into());
            };
            let limits = budget.admit(table.grid().table(), check)?;
            entry.insert(DeclaredTableGeometry::from_prepared(&table, limits, check)?);
        }
        let geometry = &tables[&id];
        let region = match target {
            FillTarget::TableBackground { .. } => geometry.bounds(),
            FillTarget::TableCell { cell, .. } => {
                let bound = geometry
                    .cell(cell)
                    .ok_or(E::Invalid("table image cell identity"))?;
                if bound.region.origin != cell {
                    return Err(E::Invalid(
                        "covered table cell has no independent image receiver",
                    ));
                }
                (bound.merged, bound.conversion_error_bound)
            }
            _ => unreachable!("table receiver targets"),
        };
        output.insert(target, region);
    }
    Ok(output)
}
