//! Source-preserving logical MCE projection (ECMA-376 Part 3, 2015).
//! No XML is rewritten here. Physical source ordinals and selected-branch
//! ancestry stay attached to projected events for later preservation planning.
mod rules;
mod stream;

use crate::{Element, ExpandedName, XmlEvent, XmlSummary};
use std::collections::BTreeSet;
pub use stream::scan;

pub const NAMESPACE: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// Trusted application/markup configuration, independent of document declarations.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub understood_namespaces: BTreeSet<String>,
    /// Processing is suspended on these elements and their complete contents.
    pub extension_elements: BTreeSet<ExpandedName>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub source_ordinal: usize,
    pub requires: Vec<String>,
    pub fallback: bool,
    pub selected: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub source_ordinal: usize,
    pub branches: Vec<Branch>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub xml: XmlSummary,
    pub selections: Vec<Selection>,
    pub ignored_elements: usize,
    pub ignored_attributes: usize,
    pub unwrapped_elements: usize,
}

pub enum Event<'a> {
    /// Observe every physical start, including inactive branches and extension
    /// payloads. This is not a semantic event; consumers must not index objects
    /// from it. Useful for preservation dependencies such as timing references.
    SourceElement {
        element: &'a Element,
        source_ordinal: usize,
    },
    Content {
        event: XmlEvent<'a>,
        /// Physical Start-event ordinal, present for projected Start events only.
        source_ordinal: Option<usize>,
        /// Physical AlternateContent ordinals surrounding this selected content.
        alternate_ancestors: &'a [usize],
        /// True on an application extension element and all of its descendants.
        extension_content: bool,
    },
}
