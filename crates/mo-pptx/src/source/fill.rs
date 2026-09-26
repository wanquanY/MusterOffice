//! Shared source-bound fill declarations for objects, lines, themes and
//! backgrounds. Reading is distinct from inheritance, brush evaluation and
//! rendering. Unresolved effects/extensions always retain physical bindings.
pub mod colors;
pub mod resolve;
mod types;
pub use types::*;
pub(super) fn is_fill(name: &mo_xml::ExpandedName) -> bool {
    name.namespace == crate::A
        && [
            "noFill",
            "solidFill",
            "gradFill",
            "pattFill",
            "blipFill",
            "grpFill",
        ]
        .contains(&name.local.as_str())
}
