//! Native cell layout attributes have their own source and defaults. They do
//! not inherit a placeholder shape's body or synthesize text-catalog nodes.
use super::*;
use crate::source::table::SourceCellAddress;

pub struct CellTextBodyResolver<'a> {
    index: &'a SourceIndex,
    surface: &'a SourceSurface,
    object: &'a SourceObject,
    reference: SourceObjectRef,
}
impl<'a> CellTextBodyResolver<'a> {
    pub fn bind(
        index: &'a SourceIndex,
        expected: &mo_common::Digest,
        reference: &SourceObjectRef,
        limits: TextBodyLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        cancelled(check)?;
        if &index.source_sha256 != expected {
            return Err(conflict());
        }
        let surface = index.surfaces.get(&reference.part).ok_or_else(conflict)?;
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
        };
        budget.bytes(reference.part.len() + 128)?;
        let mut object = None;
        for candidate in &surface.objects {
            budget.step()?;
            if candidate.native_id == reference.native_id && object.replace(candidate).is_some() {
                return Err(conflict());
            }
        }
        let object = object.filter(|o| o.table.is_some()).ok_or_else(conflict)?;
        Ok(Self {
            index,
            surface,
            object,
            reference: reference.clone(),
        })
    }
    pub fn bind_prepared(
        table: &crate::source::prepared::PreparedSourceTable<'a>,
        limits: TextBodyLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
        };
        budget.step()?;
        budget.bytes(table.part().len() + 128)?;
        Ok(Self {
            index: table.index(),
            surface: table.surface(),
            object: table.object(),
            reference: table.reference(),
        })
    }
    pub fn resolve(
        &self,
        cell: SourceCellAddress,
        limits: TextBodyLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TextBodyOutcome, PptxError> {
        cancelled(check)?;
        if limits.max_queries == 0 {
            return Err(PptxError::Limit("text body queries"));
        }
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
        };
        let result = self.compute(cell, &mut budget);
        match result {
            Ok(body) => Ok(TextBodyOutcome::Resolved {
                body: Box::new(body),
            }),
            Err(Failure::Unresolved(reason)) => Ok(TextBodyOutcome::Unresolved { reason }),
            Err(Failure::Abort(e)) => Err(e),
        }
    }
    fn compute(
        &self,
        cell: SourceCellAddress,
        budget: &mut Budget<'_>,
    ) -> Result<EffectiveTextBody, Failure> {
        budget.step()?;
        budget.bytes(
            self.reference
                .part
                .len()
                .checked_mul(24)
                .and_then(|n| n.checked_add(2048))
                .ok_or(PptxError::Limit("cell body origin bytes"))?,
        )?;
        let table = self.object.table.as_ref().ok_or_else(conflict)?;
        let native = table
            .rows
            .get(cell.row as usize)
            .and_then(|r| r.cells.get(cell.column as usize))
            .ok_or_else(conflict)?;
        let origin = |source_ordinal| TextBodyOrigin::Cell {
            object: self.reference.clone(),
            cell,
            source_ordinal,
        };
        if let Some(&at) = table.retained_ordinals.first() {
            return Err(TextBodyUnresolved::RetainedContent { origin: origin(at) }.into());
        }
        let binding = bind_body(self.object, &self.surface.text, Some(cell))?
            .ok_or(TextBodyUnresolved::NoTextBody {})?;
        let catalog = &self.surface.text;
        let root = binding.root.source_ordinal;
        if let Some(&at) = node(catalog, root)?.retained_ordinals.first() {
            return Err(TextBodyUnresolved::RetainedContent { origin: origin(at) }.into());
        }
        let id = child(catalog, root, NativeTextElement::BodyPr, budget)?.ok_or_else(conflict)?;
        let mut partial = Partial::default();
        // tcPr owns these eight attributes, including the cell-specific default
        // for horizontal overflow. bodyPr continues to supply the other fields.
        let default_origin = TextBodyOrigin::CellDefault {
            object: self.reference.clone(),
            cell,
        };
        let defaults = types::defaults();
        let cell_defaults = SourceTextBodyAttributes {
            left_inset: defaults.left_inset,
            right_inset: defaults.right_inset,
            top_inset: defaults.top_inset,
            bottom_inset: defaults.bottom_inset,
            vertical: defaults.vertical,
            anchor: defaults.anchor,
            center_anchor: defaults.center_anchor,
            horizontal_overflow: Some(NativeTextHorizontalOverflow::Clip),
            ..Default::default()
        };
        if let Some(p) = &native.properties {
            let explicit = SourceTextBodyAttributes {
                left_inset: p.margins.left.clone(),
                right_inset: p.margins.right.clone(),
                top_inset: p.margins.top.clone(),
                bottom_inset: p.margins.bottom.clone(),
                vertical: p.vertical,
                anchor: p.vertical_alignment,
                center_anchor: p.center_anchor,
                horizontal_overflow: p.horizontal_overflow,
                ..Default::default()
            };
            for value in [
                &explicit.left_inset,
                &explicit.right_inset,
                &explicit.top_inset,
                &explicit.bottom_inset,
            ]
            .into_iter()
            .flatten()
            {
                budget.bytes(value.lexical().len())?;
            }
            types::inherit(
                &mut partial.values,
                &explicit,
                &mut partial.origins,
                &origin(p.source_ordinal),
            );
        }
        types::inherit(
            &mut partial.values,
            &cell_defaults,
            &mut partial.origins,
            &default_origin,
        );
        partial.merge(catalog, id, origin(id), budget)?;
        // Share the existing theme-default resolver. No shape-template object
        // is put into this context, and no other table's body is consulted.
        Resolver {
            index: self.index,
            surface: self.surface,
            objects: BTreeMap::new(),
        }
        .theme(&mut partial, budget)?;
        budget.step()?;
        Ok(partial.finish())
    }
}
