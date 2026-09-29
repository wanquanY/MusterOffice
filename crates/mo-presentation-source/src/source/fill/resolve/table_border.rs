//! Real cell-line and table-style-line paint. Geometry/stroke properties are
//! resolved separately; every fill family uses the common fill merge engine.
use super::*;
use crate::source::table::{TableStyleEdge, styles::*};

impl Resolver<'_> {
    fn border_owner(
        &self,
        owner: &FillOwner,
        region: TableStyleRegion,
        edge: TableStyleEdge,
        budget: &mut Budget<'_>,
    ) -> Result<FillOwner, PptxError> {
        let mut out = budget.owner(owner)?;
        out.target = FillTarget::TableStyleBorder {
            native_id: native_id(&owner.target).expect("table id"),
            region,
            edge,
        };
        Ok(out)
    }
    fn border_direct(
        &self,
        line: &line::SourceLine,
        origin: &FillOrigin,
        context: Option<&FillOwner>,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        if partial.complete() {
            return Ok(());
        }
        self.retained(origin, &line.retained_ordinals, budget)?;
        if let Some(fill) = &line.fill {
            let (ordinal, input) = line_input(fill);
            let at = budget.at(origin, ordinal)?;
            partial.merge(input, &line.retained_ordinals, &at, context, budget)?;
        }
        Ok(())
    }
    fn border_style(
        &self,
        owner: &FillOwner,
        line: &SourceTableStyleLine,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        match line {
            SourceTableStyleLine::Direct { line } => {
                let origin = self.reference_origin(owner, line.source_ordinal, budget)?;
                self.border_direct(line, &origin, None, partial, budget)
            }
            SourceTableStyleLine::Reference { reference } => self.theme(
                owner,
                Reference {
                    ordinal: reference.source_ordinal,
                    index: reference.index,
                    retained: &reference.retained_ordinals,
                    line: true,
                },
                partial,
                budget,
            ),
        }
    }
    pub(super) fn table_border_fill(
        &self,
        owner: &FillOwner,
        bound: &table::Binding<'_>,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        match owner.target {
            FillTarget::TableCellBorder { cell, edge, .. } => {
                let layers = match bound.style.border(&bound.grid, cell, edge, budget.check) {
                    Ok(layers) => layers,
                    Err(reason) => return self.table_selection(owner, reason, budget),
                };
                let selected = layers.iter().rev().find(|layer| layer.line.is_some());
                let context = match selected {
                    Some(layer)
                        if matches!(layer.line, Some(SourceTableStyleLine::Reference { .. })) =>
                    {
                        Some(self.border_owner(owner, layer.region, layer.edge, budget)?)
                    }
                    _ => None,
                };
                if let Some(line) = bound
                    .grid
                    .cell(cell)
                    .and_then(|c| c.properties.as_ref())
                    .and_then(|p| p.borders[edge.index()].as_ref())
                {
                    let origin = self.declaration(owner, line.source_ordinal, budget)?;
                    self.border_direct(line, &origin, context.as_ref(), partial, budget)?;
                }
                for layer in layers.iter().rev() {
                    budget.step()?;
                    if partial.complete() {
                        break;
                    }
                    let at = self.border_owner(owner, layer.region, layer.edge, budget)?;
                    let origin = self.reference_origin(&at, layer.part.source_ordinal, budget)?;
                    self.retained(&origin, &layer.part.retained_ordinals, budget)?;
                    if let Some(line) = layer.line {
                        self.border_style(&at, line, partial, budget)?;
                    }
                }
            }
            FillTarget::TableStyleBorder { region, edge, .. } => {
                if let Some((style, _)) = bound.style.definition()
                    && let Some(part) = style.parts.get(&region)
                {
                    let origin = self.reference_origin(owner, part.source_ordinal, budget)?;
                    self.retained(&origin, &part.retained_ordinals, budget)?;
                    if let Some(line) = part
                        .cell
                        .as_ref()
                        .and_then(|c| c.borders.as_ref())
                        .and_then(|b| b.edges[edge.index()].as_ref())
                    {
                        self.border_style(owner, line, partial, budget)?;
                    }
                }
            }
            _ => return Err(conflict("non-border table fill target").into()),
        }
        Ok(())
    }
}
