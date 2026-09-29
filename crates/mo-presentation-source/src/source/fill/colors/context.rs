//! On-demand native style context. No format-style lookup or eager color sampling.
use super::*;
use crate::source::table::styles::*;
use crate::source::{SourceObject, drawingml::SourceColor, fill::SourceBackgroundDefinition};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct Reference<'a> {
    pub part: Option<&'a str>,
    pub ordinal: u32,
    pub color: Option<&'a SourceColor>,
    pub retained: Option<u32>,
}
#[derive(Default)]
pub(super) struct Contexts<'a> {
    // Each source object vector is traversed at most once per consulted part.
    objects: BTreeMap<&'a str, BTreeMap<u32, &'a SourceObject>>,
    table_references: BTreeMap<(&'a str, FillTarget), Option<Reference<'a>>>,
}
impl<'a> Contexts<'a> {
    pub fn get(
        &mut self,
        index: &'a SourceIndex,
        owner: &FillOwner,
        budget: &mut color::Budget<'_>,
    ) -> color::Computed<Option<Reference<'a>>> {
        budget.step()?;
        let invalid = || {
            PptxError::SourceConflict("fill color context is not bound to inspected source".into())
        };
        let (part, surface) = index
            .surfaces
            .get_key_value(&owner.part)
            .ok_or_else(invalid)?;
        let reference = match owner.target {
            FillTarget::Object { native_id }
            | FillTarget::Line { native_id }
            | FillTarget::TableStyleFill { native_id, .. }
            | FillTarget::TableStyleBorder { native_id, .. } => {
                if !self.objects.contains_key(part.as_str()) {
                    let mut objects = BTreeMap::new();
                    for object in &surface.objects {
                        budget.step()?;
                        if objects.insert(object.native_id, object).is_some() {
                            return Err(invalid().into());
                        }
                    }
                    self.objects.insert(part.as_str(), objects);
                }
                let object = self.objects[part.as_str()]
                    .get(&native_id)
                    .ok_or_else(invalid)?;
                if matches!(
                    owner.target,
                    FillTarget::TableStyleFill { .. } | FillTarget::TableStyleBorder { .. }
                ) {
                    let key = (part.as_str(), owner.target.clone());
                    if let Some(reference) = self.table_references.get(&key) {
                        return Ok(*reference);
                    }
                    let table = object.table.as_ref().ok_or_else(invalid)?;
                    let bound =
                        BoundTableStyle::bind(table, index.table_styles.as_deref(), &|| {
                            budget.is_cancelled()
                        })
                        .map_err(|e| {
                            if e == TableStyleSelectionError::Cancelled {
                                PptxError::Cancelled
                            } else {
                                invalid()
                            }
                        })?;
                    let (style, location) = bound.definition().ok_or_else(invalid)?;
                    let part = match location {
                        TableStyleLocation::Inline => part.as_str(),
                        TableStyleLocation::Shared(catalog) => catalog.part.as_str(),
                    };
                    let reference = table_reference(style, &owner.target, part);
                    self.table_references.insert(key, reference);
                    reference
                } else if matches!(owner.target, FillTarget::Line { .. }) {
                    object.line_reference.as_ref().map(|r| Reference {
                        part: None,
                        ordinal: r.source_ordinal,
                        color: r.color.as_ref(),
                        retained: r.retained_ordinals.first().copied(),
                    })
                } else {
                    object.fill_reference.as_ref().map(|r| Reference {
                        part: None,
                        ordinal: r.source_ordinal,
                        color: r.color.as_ref(),
                        retained: r.retained_ordinals.first().copied(),
                    })
                }
            }
            FillTarget::Background {} => {
                match &surface.background.as_ref().ok_or_else(invalid)?.definition {
                    SourceBackgroundDefinition::Reference(r) => Some(Reference {
                        part: None,
                        ordinal: r.source_ordinal,
                        color: r.color.as_ref(),
                        retained: r.retained_ordinals.first().copied(),
                    }),
                    SourceBackgroundDefinition::Properties { .. } => None,
                }
            }
            FillTarget::RootGroup {}
            | FillTarget::Picture { .. }
            | FillTarget::TableCell { .. }
            | FillTarget::TableCellBorder { .. }
            | FillTarget::TableBackground { .. } => None,
        };
        Ok(reference)
    }
}

fn table_reference<'a>(
    style: &'a SourceTableStyle,
    target: &FillTarget,
    part: &'a str,
) -> Option<Reference<'a>> {
    match *target {
        FillTarget::TableStyleFill { region, .. } => {
            let (fill, retained) = match region {
                Some(region) => style
                    .parts
                    .get(&region)
                    .map(|p| {
                        (
                            p.cell.as_ref().and_then(|c| c.fill.as_ref()),
                            p.retained_ordinals.first().copied(),
                        )
                    })
                    .unwrap_or_default(),
                None => style
                    .background
                    .as_ref()
                    .map(|bg| (bg.fill.as_ref(), bg.retained_ordinals.first().copied()))
                    .unwrap_or_default(),
            };
            match fill {
                Some(SourceTableStyleFill::Reference { reference: r }) => Some(Reference {
                    part: Some(part),
                    ordinal: r.source_ordinal,
                    color: r.color.as_ref(),
                    retained: retained.or_else(|| r.retained_ordinals.first().copied()),
                }),
                _ => None,
            }
        }
        FillTarget::TableStyleBorder { region, edge, .. } => {
            let p = style.parts.get(&region)?;
            let border = p.cell.as_ref()?.borders.as_ref()?.edges[edge.index()].as_ref()?;
            match border {
                SourceTableStyleLine::Reference { reference: r } => Some(Reference {
                    part: Some(part),
                    ordinal: r.source_ordinal,
                    color: r.color.as_ref(),
                    retained: p
                        .retained_ordinals
                        .first()
                        .or(r.retained_ordinals.first())
                        .copied(),
                }),
                _ => None,
            }
        }
        _ => None,
    }
}
