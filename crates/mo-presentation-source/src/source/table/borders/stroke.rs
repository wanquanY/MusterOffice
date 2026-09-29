use super::*;
use crate::source::{SourceSurface, line::SourceLine};

enum Failure {
    Unresolved(LineUnresolved),
    Abort(PptxError),
}
impl From<PptxError> for Failure {
    fn from(e: PptxError) -> Self {
        Self::Abort(e)
    }
}
impl From<LineUnresolved> for Failure {
    fn from(e: LineUnresolved) -> Self {
        Self::Unresolved(e)
    }
}
fn selection(object: &SourceObjectRef, reason: TableStyleSelectionError) -> Failure {
    if reason == TableStyleSelectionError::Cancelled {
        return PptxError::Cancelled.into();
    }
    LineUnresolved::TableStyle {
        object: object.clone(),
        reason,
    }
    .into()
}
fn merge(
    partial: &mut GeometryPartial,
    line: &SourceLine,
    origin: LineOrigin,
    budget: &mut LineBudget<'_>,
) -> Result<(), Failure> {
    let bytes = match &origin {
        LineOrigin::TableCell { object, .. } => object.part.len(),
        LineOrigin::TableStyle { part, object, .. } => part
            .len()
            .checked_add(object.part.len())
            .ok_or(PptxError::Limit("table line origin bytes"))?,
        LineOrigin::TableTheme { part, via, .. } => part
            .len()
            .checked_add(via.part.len())
            .ok_or(PptxError::Limit("table line origin bytes"))?,
        _ => return Err(PptxError::SourceConflict("table line declaration origin".into()).into()),
    };
    budget.bytes(
        bytes
            .checked_mul(64)
            .ok_or(PptxError::Limit("table line origin bytes"))?,
    )?;
    budget.line(line)?;
    partial.merge(line, &origin)?;
    Ok(())
}

pub(super) fn resolve(
    index: &SourceIndex,
    surface: &SourceSurface,
    object: &SourceObjectRef,
    target: &TableBorderTarget,
    bound: &TablePaintBinding<'_>,
    budget: &mut LineBudget<'_>,
) -> Result<LineGeometryOutcome, PptxError> {
    let result = (|| -> Result<EffectiveLineGeometry, Failure> {
        if let Some(&source_ordinal) = bound.grid.table().retained_ordinals.first() {
            return Err(LineUnresolved::RetainedContent {
                origin: LineOrigin::Object {
                    object: object.clone(),
                    source_ordinal,
                },
            }
            .into());
        }
        let layers = bound
            .style
            .border(&bound.grid, target.cell, target.edge, budget.check)
            .map_err(|e| selection(object, e))?;
        let mut partial = GeometryPartial::default();
        if let Some(line) = bound
            .grid
            .cell(target.cell)
            .and_then(|c| c.properties.as_ref())
            .and_then(|p| p.borders[target.edge.index()].as_ref())
        {
            merge(
                &mut partial,
                line,
                LineOrigin::TableCell {
                    object: object.clone(),
                    cell: target.cell,
                    edge: target.edge,
                    source_ordinal: line.source_ordinal,
                },
                budget,
            )?;
        }
        if let Some((_, location)) = bound.style.definition() {
            let part = match location {
                TableStyleLocation::Inline => &object.part,
                TableStyleLocation::Shared(catalog) => &catalog.part,
            };
            for layer in layers.iter().rev() {
                budget.step()?;
                if partial.complete() {
                    break;
                }
                if let Some(&ordinal) = layer.part.retained_ordinals.first() {
                    return Err(LineUnresolved::RetainedContent {
                        origin: LineOrigin::TableStyle {
                            part: part.clone(),
                            object: object.clone(),
                            region: layer.region,
                            edge: layer.edge,
                            source_ordinal: ordinal,
                        },
                    }
                    .into());
                }
                match layer.line {
                    None => (),
                    Some(SourceTableStyleLine::Direct { line }) => merge(
                        &mut partial,
                        line,
                        LineOrigin::TableStyle {
                            part: part.clone(),
                            object: object.clone(),
                            region: layer.region,
                            edge: layer.edge,
                            source_ordinal: line.source_ordinal,
                        },
                        budget,
                    )?,
                    Some(SourceTableStyleLine::Reference { reference }) => {
                        if let Some(&ordinal) = reference.retained_ordinals.first() {
                            return Err(LineUnresolved::RetainedContent {
                                origin: LineOrigin::TableStyle {
                                    part: part.clone(),
                                    object: object.clone(),
                                    region: layer.region,
                                    edge: layer.edge,
                                    source_ordinal: ordinal,
                                },
                            }
                            .into());
                        }
                        if reference.index == 0 {
                            continue;
                        }
                        let binding = surface.theme_selection.format.as_ref().ok_or_else(|| {
                            LineUnresolved::MissingFormatScheme {
                                object: object.clone(),
                            }
                        })?;
                        let scheme = index
                            .themes
                            .get(&binding.part)
                            .and_then(|t| t.format_scheme.as_ref())
                            .filter(|s| s.source_ordinal == binding.source_ordinal)
                            .ok_or_else(|| {
                                PptxError::SourceConflict(
                                    "table line format binding differs from inspected source"
                                        .into(),
                                )
                            })?;
                        let entry = scheme
                            .lines
                            .get((reference.index - 1) as usize)
                            .ok_or_else(|| LineUnresolved::StyleIndexOutOfRange {
                                object: object.clone(),
                                index: reference.index,
                                available: scheme.lines.len() as u32,
                            })?;
                        let line = entry
                            .line
                            .as_ref()
                            .filter(|l| l.source_ordinal == entry.source_ordinal)
                            .ok_or_else(|| {
                                PptxError::SourceConflict(
                                    "table line theme binding differs from inspected source".into(),
                                )
                            })?;
                        merge(
                            &mut partial,
                            line,
                            LineOrigin::TableTheme {
                                part: binding.part.clone(),
                                source_ordinal: line.source_ordinal,
                                via: object.clone(),
                                region: layer.region,
                                edge: layer.edge,
                                reference_ordinal: reference.source_ordinal,
                                style_index: reference.index,
                            },
                            budget,
                        )?;
                    }
                }
            }
        }
        Ok(partial.finish())
    })();
    match result {
        Ok(geometry) => Ok(LineGeometryOutcome::Resolved {
            geometry: Box::new(geometry),
        }),
        Err(Failure::Unresolved(reason)) => Ok(LineGeometryOutcome::Unresolved { reason }),
        Err(Failure::Abort(error)) => Err(error),
    }
}
