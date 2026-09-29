//! Table paint uses the same fill merge/format-matrix engine as shape paint.
//! Owners name native cells or style regions; shared declarations retain their
//! actual part. No synthetic shape, geometry, text body or image is introduced.
use super::*;
use crate::source::table::{grid::*, styles::*};

#[derive(Clone)]
pub(in crate::source) struct Binding<'a> {
    pub(in crate::source) grid: std::sync::Arc<NativeTableGrid<'a>>,
    pub(in crate::source) style: std::sync::Arc<BoundTableStyle<'a>>,
}
#[derive(Clone)]
pub(in crate::source) enum BindIssue {
    Grid(NativeTableGridIssue),
    Style(TableStyleSelectionError),
}
pub(in crate::source) type Bindings<'a> = BTreeMap<(&'a str, u32), Result<Binding<'a>, BindIssue>>;

pub(super) fn is_target(target: &FillTarget) -> bool {
    matches!(
        target,
        FillTarget::TableCell { .. }
            | FillTarget::TableCellBorder { .. }
            | FillTarget::TableBackground { .. }
            | FillTarget::TableStyleFill { .. }
            | FillTarget::TableStyleBorder { .. }
    )
}

/// Each table's topology is compiled once per batch. Reserve its worst-case
/// traversal and temporary owner/region/id storage in the shared query budget,
/// so many small tables cannot bypass a per-table limit.
pub(super) fn prepare<'a>(
    index: &'a SourceIndex,
    surface: &str,
    targets: &[FillTarget],
    objects: &crate::source::prepared::ObjectBindings<'a>,
    mut preparation: Option<&mut crate::source::prepared::SourcePreparation<'a>>,
    budget: &mut Budget<'_>,
) -> Result<Bindings<'a>, PptxError> {
    let surface = index
        .surfaces
        .get_key_value(surface)
        .ok_or_else(|| conflict("table fill surface binding"))?
        .0
        .as_str();
    let mut out = BTreeMap::new();
    for target in targets.iter().filter(|t| is_target(t)) {
        budget.step()?;
        let id = native_id(target).expect("table native id");
        let Some((key, object)) = objects.get_key_value(&(surface, id)) else {
            continue;
        };
        if out.contains_key(&key) || object.kind != SourceObjectKind::GraphicFrame {
            continue;
        }
        let Some(table) = &object.table else { continue };
        let count = table
            .columns
            .len()
            .checked_mul(table.rows.len())
            .ok_or(PptxError::Limit("fill table cells"))?;
        let steps = count
            .checked_mul(2)
            .and_then(|n| n.checked_add(table.rows.len()))
            .ok_or(PptxError::Limit("fill table steps"))?;
        budget.steps(steps)?;
        budget.values(
            count
                .checked_mul(4)
                .ok_or(PptxError::Limit("fill table values"))?,
        )?;
        if let Some(preparation) = preparation.as_deref_mut() {
            use crate::source::prepared::SourceTablePreparation;
            // Cache hits still consume the query's logical grid allowance;
            // reject undersized query budgets before allocating a new grid.
            budget.steps(count)?;
            let prepared = preparation.table(
                &SourceObjectRef {
                    part: surface.into(),
                    native_id: id,
                },
                budget.check,
            )?;
            budget.bytes(prepared.cost().id_bytes)?;
            let result = match prepared {
                SourceTablePreparation::InvalidGrid { reason, .. } => Err(BindIssue::Grid(reason)),
                SourceTablePreparation::Prepared { table } => table
                    .style()
                    .map(|style| Binding {
                        grid: table.shared_grid(),
                        style,
                    })
                    .map_err(BindIssue::Style),
            };
            out.insert(key, result);
            continue;
        }

        let mut id_bytes = 0usize;
        for row in &table.rows {
            for cell in &row.cells {
                budget.step()?;
                if let Some(id) = &cell.native_id {
                    budget.bytes(id.len())?;
                    id_bytes = id_bytes
                        .checked_add(id.len())
                        .ok_or(PptxError::Limit("fill table ids"))?;
                }
            }
        }
        let grid = match NativeTableGrid::compile(
            table,
            NativeTableGridLimits {
                max_cells: count,
                max_steps: steps,
                max_id_bytes: id_bytes,
            },
            budget.check,
        ) {
            Ok(grid) => grid,
            Err(NativeTableGridError::Invalid(reason)) => {
                out.insert(key, Err(BindIssue::Grid(reason)));
                continue;
            }
            Err(NativeTableGridError::Limit) => return Err(PptxError::Limit("fill table grid")),
            Err(NativeTableGridError::Cancelled) => return Err(PptxError::Cancelled),
        };
        let style = match BoundTableStyle::bind(table, index.table_styles.as_deref(), budget.check)
        {
            Ok(style) => style,
            Err(TableStyleSelectionError::Cancelled) => return Err(PptxError::Cancelled),
            Err(reason) => {
                out.insert(key, Err(BindIssue::Style(reason)));
                continue;
            }
        };
        out.insert(
            key,
            Ok(Binding {
                grid: std::sync::Arc::new(grid),
                style: std::sync::Arc::new(style),
            }),
        );
    }
    Ok(out)
}

impl<'a> Resolver<'a> {
    pub(super) fn table_binding(
        &self,
        owner: &FillOwner,
        budget: &mut Budget<'_>,
    ) -> Result<&Binding<'a>, Failure> {
        budget.step()?;
        let id = native_id(&owner.target).ok_or_else(|| conflict("table fill identity"))?;
        let part = self
            .index
            .surfaces
            .get_key_value(&owner.part)
            .ok_or_else(|| conflict("table fill surface"))?
            .0
            .as_str();
        match self.tables.get(&(part, id)) {
            Some(Ok(bound)) => Ok(bound),
            Some(Err(BindIssue::Grid(reason))) => Err(FillUnresolved::TableGrid {
                owner: budget.owner(owner)?,
                reason: reason.clone(),
            }
            .into()),
            Some(Err(BindIssue::Style(reason))) => {
                self.table_selection(owner, reason.clone(), budget)
            }
            None => Err(FillUnresolved::UnsupportedTarget {
                owner: budget.owner(owner)?,
            }
            .into()),
        }
    }
    pub(super) fn table_selection<T>(
        &self,
        owner: &FillOwner,
        reason: TableStyleSelectionError,
        budget: &mut Budget<'_>,
    ) -> Result<T, Failure> {
        if reason == TableStyleSelectionError::Cancelled {
            return Err(PptxError::Cancelled.into());
        }
        Err(FillUnresolved::TableStyle {
            owner: budget.owner(owner)?,
            reason,
        }
        .into())
    }
    pub(super) fn reference_origin(
        &self,
        owner: &FillOwner,
        ordinal: u32,
        budget: &mut Budget<'_>,
    ) -> Result<FillOrigin, Failure> {
        if matches!(
            owner.target,
            FillTarget::TableStyleFill { .. } | FillTarget::TableStyleBorder { .. }
        ) {
            let bound = self.table_binding(owner, budget)?;
            let (_, location) = bound
                .style
                .definition()
                .ok_or_else(|| conflict("missing table style declaration"))?;
            let part = match location {
                TableStyleLocation::Inline => &owner.part,
                TableStyleLocation::Shared(catalog) => &catalog.part,
            };
            budget.bytes(part.len())?;
            Ok(FillOrigin::TableStyle {
                part: part.clone(),
                source_ordinal: ordinal,
                via: budget.owner(owner)?,
            })
        } else {
            Ok(self.declaration(owner, ordinal, budget)?)
        }
    }
    fn table_direct(
        &self,
        fill: &SourceFill,
        origin: &FillOrigin,
        context: Option<&FillOwner>,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        partial.merge(
            (&fill.definition).into(),
            &fill.retained_ordinals,
            origin,
            context,
            budget,
        )
    }
    fn style_fill(
        &self,
        owner: &FillOwner,
        fill: &SourceTableStyleFill,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        match fill {
            SourceTableStyleFill::Direct { fill } => {
                let origin = self.reference_origin(owner, fill.source_ordinal, budget)?;
                self.table_direct(fill, &origin, None, partial, budget)
            }
            SourceTableStyleFill::Reference { reference } => self.theme(
                owner,
                Reference {
                    ordinal: reference.source_ordinal,
                    index: reference.index,
                    retained: &reference.retained_ordinals,
                    line: false,
                },
                partial,
                budget,
            ),
        }
    }
    fn table_style_owner(
        &self,
        owner: &FillOwner,
        region: Option<TableStyleRegion>,
        budget: &mut Budget<'_>,
    ) -> Result<FillOwner, PptxError> {
        let mut out = budget.owner(owner)?;
        out.target = FillTarget::TableStyleFill {
            native_id: native_id(&owner.target).expect("table id"),
            region,
        };
        Ok(out)
    }
    pub(super) fn retained(
        &self,
        origin: &FillOrigin,
        ordinals: &[u32],
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        if let Some(&ordinal) = ordinals.first() {
            return Err(FillUnresolved::RetainedContent {
                origin: budget.at(origin, ordinal)?,
            }
            .into());
        }
        Ok(())
    }
    fn table_effects(
        &self,
        origin: &FillOrigin,
        effects: &crate::source::effects::SourceEffectProperties,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        self.retained(origin, &effects.retained_ordinals, budget)?;
        if !effects.is_explicitly_empty_list() {
            return Err(FillUnresolved::EffectEvaluationRequired {
                origin: budget.at(origin, effects.source_ordinal)?,
            }
            .into());
        }
        Ok(())
    }
    pub(super) fn table_fill(
        &self,
        owner: &FillOwner,
        budget: &mut Budget<'_>,
    ) -> Result<EffectiveFill, Failure> {
        let bound = self.table_binding(owner, budget)?;
        let table = bound.grid.table();
        let table_origin = self.declaration(owner, table.source_ordinal, budget)?;
        self.retained(&table_origin, &table.retained_ordinals, budget)?;
        if let Some((style, location)) = bound.style.definition() {
            let style_owner = self.table_style_owner(owner, None, budget)?;
            let origin = self.reference_origin(&style_owner, style.source_ordinal, budget)?;
            self.retained(&origin, &style.retained_ordinals, budget)?;
            if let TableStyleLocation::Shared(catalog) = location {
                self.retained(&origin, &catalog.retained_ordinals, budget)?;
            }
        }
        let mut partial = merge::Partial::default();
        match owner.target {
            FillTarget::TableCellBorder { .. } | FillTarget::TableStyleBorder { .. } => {
                self.table_border_fill(owner, bound, &mut partial, budget)?;
            }
            FillTarget::TableCell { cell, .. } => {
                let layers = match bound.style.cell(&bound.grid, cell, budget.check) {
                    Ok(layers) => layers,
                    Err(reason) => return self.table_selection(owner, reason, budget),
                };
                let selected = layers.iter().rev().find_map(|layer| {
                    layer
                        .part
                        .cell
                        .as_ref()?
                        .fill
                        .as_ref()
                        .map(|fill| (layer.region, fill))
                });
                let context = match selected {
                    Some((region, SourceTableStyleFill::Reference { .. })) => {
                        Some(self.table_style_owner(owner, Some(region), budget)?)
                    }
                    _ => None,
                };
                if let Some(fill) = bound
                    .grid
                    .cell(cell)
                    .and_then(|c| c.properties.as_ref())
                    .and_then(|p| p.fill.as_ref())
                {
                    let origin = self.declaration(owner, fill.source_ordinal, budget)?;
                    self.table_direct(fill, &origin, context.as_ref(), &mut partial, budget)?;
                }
                for layer in layers.iter().rev() {
                    budget.step()?;
                    if partial.complete() {
                        break;
                    }
                    let at = self.table_style_owner(owner, Some(layer.region), budget)?;
                    let origin = self.reference_origin(&at, layer.part.source_ordinal, budget)?;
                    self.retained(&origin, &layer.part.retained_ordinals, budget)?;
                    if let Some(fill) = layer.part.cell.as_ref().and_then(|c| c.fill.as_ref()) {
                        self.style_fill(&at, fill, &mut partial, budget)?;
                    }
                }
            }
            FillTarget::TableBackground { .. } => {
                let at = self.table_style_owner(owner, None, budget)?;
                let background = bound
                    .style
                    .definition()
                    .and_then(|(s, _)| s.background.as_ref());
                let context = background
                    .and_then(|bg| bg.fill.as_ref())
                    .filter(|fill| matches!(fill, SourceTableStyleFill::Reference { .. }))
                    .map(|_| &at);
                if let Some(properties) = &table.properties {
                    if let Some(effects) = &properties.effects {
                        self.table_effects(&table_origin, effects, budget)?;
                    }
                    if let Some(fill) = &properties.fill {
                        let origin = self.declaration(owner, fill.source_ordinal, budget)?;
                        self.table_direct(fill, &origin, context, &mut partial, budget)?;
                    }
                }
                self.style_background(&at, background, &mut partial, budget)?;
            }
            FillTarget::TableStyleFill { region, .. } => {
                if let Some((style, _)) = bound.style.definition() {
                    if let Some(region) = region {
                        if let Some(part) = style.parts.get(&region) {
                            let origin =
                                self.reference_origin(owner, part.source_ordinal, budget)?;
                            self.retained(&origin, &part.retained_ordinals, budget)?;
                            if let Some(fill) = part.cell.as_ref().and_then(|c| c.fill.as_ref()) {
                                self.style_fill(owner, fill, &mut partial, budget)?;
                            }
                        }
                    } else {
                        self.style_background(
                            owner,
                            style.background.as_ref(),
                            &mut partial,
                            budget,
                        )?;
                    }
                }
            }
            _ => return Err(conflict("non-table fill target").into()),
        }
        partial.finish(budget)
    }
    fn style_background(
        &self,
        owner: &FillOwner,
        background: Option<&SourceTableBackgroundStyle>,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        if let Some(bg) = background {
            let origin = self.reference_origin(owner, bg.source_ordinal, budget)?;
            self.retained(&origin, &bg.retained_ordinals, budget)?;
            if let Some(effects) = &bg.effects {
                match effects {
                    SourceTableStyleEffects::Direct { effects } => {
                        self.table_effects(&origin, effects, budget)?
                    }
                    SourceTableStyleEffects::Reference { reference } => {
                        self.retained(&origin, &reference.retained_ordinals, budget)?;
                        if reference.index != 0 {
                            return Err(FillUnresolved::EffectEvaluationRequired {
                                origin: budget.at(&origin, reference.source_ordinal)?,
                            }
                            .into());
                        }
                    }
                }
            }
            if !partial.complete()
                && let Some(fill) = &bg.fill
            {
                self.style_fill(owner, fill, partial, budget)?;
            }
        }
        Ok(())
    }
}
