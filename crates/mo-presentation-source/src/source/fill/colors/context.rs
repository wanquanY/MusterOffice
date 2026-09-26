//! On-demand native style context. No format-style lookup or eager color sampling.
use super::*;
use crate::source::{SourceObject, drawingml::SourceColor, fill::SourceBackgroundDefinition};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct Reference<'a> {
    pub ordinal: u32,
    pub color: Option<&'a SourceColor>,
    pub retained: Option<u32>,
}
#[derive(Default)]
pub(super) struct Contexts<'a> {
    // Each source object vector is traversed at most once per consulted part.
    objects: BTreeMap<&'a str, BTreeMap<u32, &'a SourceObject>>,
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
            FillTarget::Object { native_id } | FillTarget::Line { native_id } => {
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
                if matches!(owner.target, FillTarget::Line { .. }) {
                    object.line_reference.as_ref().map(|r| Reference {
                        ordinal: r.source_ordinal,
                        color: r.color.as_ref(),
                        retained: r.retained_ordinals.first().copied(),
                    })
                } else {
                    object.fill_reference.as_ref().map(|r| Reference {
                        ordinal: r.source_ordinal,
                        color: r.color.as_ref(),
                        retained: r.retained_ordinals.first().copied(),
                    })
                }
            }
            FillTarget::Background {} => {
                match &surface.background.as_ref().ok_or_else(invalid)?.definition {
                    SourceBackgroundDefinition::Reference(r) => Some(Reference {
                        ordinal: r.source_ordinal,
                        color: r.color.as_ref(),
                        retained: r.retained_ordinals.first().copied(),
                    }),
                    SourceBackgroundDefinition::Properties { .. } => None,
                }
            }
            FillTarget::RootGroup {} | FillTarget::Picture { .. } => None,
        };
        Ok(reference)
    }
}
