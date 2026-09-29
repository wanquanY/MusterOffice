//! Construction-only root identity index; no cache or host state is serialized.
use super::*;
use crate::source::malformed;
use mo_xml::XmlError;
use std::collections::BTreeSet;
#[derive(Default)]
pub(in crate::source) struct CatalogRoots(
    BTreeSet<(
        Option<u32>,
        Option<crate::source::table::SourceCellAddress>,
        NativeTextElement,
    )>,
);
impl CatalogRoots {
    pub fn existing(catalog: &SourceTextCatalog) -> Result<Self, XmlError> {
        let mut roots = Self::default();
        roots.include(catalog)?;
        Ok(roots)
    }
    fn include(&mut self, catalog: &SourceTextCatalog) -> Result<(), XmlError> {
        if catalog
            .roots
            .windows(2)
            .any(|w| w[0].source_ordinal >= w[1].source_ordinal)
        {
            return Err(malformed("unordered native text roots"));
        }
        for r in &catalog.roots {
            let node = catalog
                .nodes
                .get(&r.source_ordinal)
                .ok_or_else(|| malformed("missing text root node"))?;
            if node.parent.is_some() || !self.0.insert((r.owner, r.cell, node.element)) {
                return Err(malformed("duplicate native text style root"));
            }
        }
        Ok(())
    }
    pub fn append(
        &mut self,
        target: &mut SourceTextCatalog,
        source: SourceTextCatalog,
    ) -> Result<(), XmlError> {
        if target
            .roots
            .last()
            .zip(source.roots.first())
            .is_some_and(|(a, b)| a.source_ordinal >= b.source_ordinal)
        {
            return Err(malformed("unordered native text roots"));
        }
        self.include(&source)?;
        for (id, node) in source.nodes {
            if target.nodes.insert(id, node).is_some() {
                return Err(malformed("duplicate text part binding"));
            }
        }
        for (id, node) in source.effect_nodes {
            if target.effect_nodes.insert(id, node).is_some() {
                return Err(malformed("duplicate text effect binding"));
            }
        }
        target.roots.extend(source.roots);
        Ok(())
    }
}
impl SourceTextCatalog {
    /// Roots are in physical discovery order (monotonic ordinals) in inspected
    /// and authored catalogs. Lookup does not scan all other table cells.
    pub fn root(&self, ordinal: u32) -> Option<&SourceTextRoot> {
        self.roots
            .binary_search_by_key(&ordinal, |r| r.source_ordinal)
            .ok()
            .map(|i| &self.roots[i])
    }
}
