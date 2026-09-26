//! Query-local preset cache. Only pinned embedded XML is parsed here; no I/O,
//! process-wide cache, host font or application dependency enters the kernel.
use super::session::Budget;
use crate::{
    PptxError,
    source::{
        SourceLimits,
        geometry::{NativeShapeType, SourceCustomGeometry, SourceGeometryDefinition, read},
    },
};
use mo_xml::{XmlError, XmlEvent, XmlLimits};
use std::collections::BTreeMap;
const DATA: &[u8] = include_bytes!("../../../../../../components/drawingml-presets/catalog.xml");
const INDEX: &[u8] =
    include_bytes!("../../../../../../components/drawingml-presets/catalog-index.bin");

#[derive(Default)]
pub(super) struct Catalog(BTreeMap<NativeShapeType, SourceCustomGeometry>);
impl Catalog {
    pub fn get(
        &mut self,
        preset: NativeShapeType,
        budget: &mut Budget<'_>,
    ) -> Result<&SourceCustomGeometry, PptxError> {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.0.entry(preset) {
            entry.insert(read(preset, budget)?);
        }
        Ok(&self.0[&preset])
    }
}
fn read(
    preset: NativeShapeType,
    budget: &mut Budget<'_>,
) -> Result<SourceCustomGeometry, PptxError> {
    let index = preset.index() * 8;
    let word =
        |i| u32::from_le_bytes(INDEX[i..i + 4].try_into().expect("pinned catalog index")) as usize;
    let xml = &DATA[word(index)..word(index) + word(index + 4)];
    budget.lexical(std::str::from_utf8(xml).expect("generated catalog UTF-8"))?;
    let mut grammar_budget = read::Budget::default();
    let mut reader = None;
    let mut ordinal = 0u32;
    let limits = SourceLimits::default();
    let check = budget.check;
    mo_xml::scan_with_control(xml, XmlLimits::default(), check, |event| {
        match event {
            XmlEvent::Start { element, depth, .. } => {
                budget.step().map_err(|e| match e {
                    PptxError::Cancelled => XmlError::Cancelled,
                    _ => XmlError::Limit("preset catalog work"),
                })?;
                if let Some(r) = &mut reader {
                    let r: &mut read::Reader = r;
                    r.start(element, depth, ordinal, false, &mut grammar_budget, limits)?;
                } else {
                    reader = Some(read::Reader::new(
                        element,
                        depth,
                        ordinal,
                        &mut grammar_budget,
                        limits,
                    )?);
                }
                ordinal += 1;
            }
            XmlEvent::End { depth, .. } => {
                let r = reader.as_mut().expect("pinned catalog root");
                if depth != r.depth {
                    r.end(depth)?;
                }
            }
            XmlEvent::Text { text, .. } => {
                if let Some(r) = &reader {
                    r.text(text)?;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let geometry = reader.expect("nonempty pinned catalog").finish()?;
    assert!(
        geometry.retained_ordinals.is_empty(),
        "catalog retained unknown content"
    );
    match geometry.definition {
        SourceGeometryDefinition::Custom(custom) => Ok(*custom),
        _ => unreachable!("catalog stores only custom definitions"),
    }
}
