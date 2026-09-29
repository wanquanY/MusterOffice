//! Bounded, namespace-aware XML events. No external entities or host I/O.
mod attribute_edit;
mod child_edit;
mod edit_bytes;
mod encoding;
pub mod mce;
mod names;
mod scan;
mod text_edit;

pub use encoding::XmlEncoding;
pub use names::ExpandedName;
pub fn is_ncname(value: &str) -> bool {
    names::ncname(value)
}
pub use attribute_edit::{AttributeEdit, AttributeRewriteLimits, rewrite_attributes};
pub use child_edit::append_child;
pub use scan::{scan, scan_with_control};
use std::ops::Range;
pub use text_edit::{TextReplacement, TextRewriteLimits, rewrite_text};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XmlError {
    #[error("XML compatibility profile mismatch: {0}")]
    Compatibility(String),
    #[error("XML edit conflict: {0}")]
    EditConflict(String),
    #[error("XML scan cancelled")]
    Cancelled,
    #[error("XML limit exceeded: {0}")]
    Limit(&'static str),
    #[error("invalid XML: {0}")]
    Malformed(String),
    #[error("XML encoding: {0}")]
    Encoding(String),
    #[error("DTD and custom entities are prohibited")]
    Dtd,
}

#[derive(Debug, Clone, Copy)]
pub struct XmlLimits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_elements: usize,
    pub max_attributes: usize,
    pub max_attribute_bytes: usize,
    pub max_text_bytes: usize,
}

impl Default for XmlLimits {
    fn default() -> Self {
        Self {
            max_bytes: 32 * 1024 * 1024,
            max_depth: 256,
            max_elements: 1_000_000,
            max_attributes: 256,
            max_attribute_bytes: 1024 * 1024,
            max_text_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: ExpandedName,
    pub qualified_name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: ExpandedName,
    pub qualified_name: String,
    pub attributes: Vec<Attribute>,
}

impl Element {
    pub fn attribute(&self, local: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.name.is("", local))
            .map(|a| a.value.as_str())
    }
}

/// Spans address the decoded UTF-8 stream (after BOM removal), not UTF-16 source bytes.
/// Start/End are emitted even for empty elements. Original package bytes stay separate.
#[derive(Debug)]
pub enum XmlEvent<'a> {
    Start {
        element: &'a Element,
        /// In-scope bindings for QName-valued attributes (MCE Requires, etc.).
        /// Borrowed from the scanner; no per-element namespace-map clone.
        namespaces: &'a std::collections::BTreeMap<String, String>,
        depth: usize,
        span: Range<usize>,
    },
    End {
        name: &'a ExpandedName,
        depth: usize,
        span: Range<usize>,
    },
    Text {
        text: &'a str,
        depth: usize,
        span: Range<usize>,
    },
    Comment {
        span: Range<usize>,
    },
    ProcessingInstruction {
        span: Range<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlSummary {
    pub encoding: XmlEncoding,
    pub elements: usize,
    pub max_depth: usize,
    pub text_bytes: usize,
}
