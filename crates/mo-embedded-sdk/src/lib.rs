//! Rust embedding surface for an existing product Runtime.
//!
//! This crate re-exports the original types and implementations. It introduces
//! no document conversion, database, queue, authorization or commit owner.
//! The host resolves authorized immutable inputs and supplies cancellation;
//! it must verify final stored results and perform its own fenced transaction.
//! Native graphics run in the separately pinned worker, not inside this SDK.
pub use mo_common as common;
pub use mo_native_export::{NativeExportCandidate, NativeExporter, executable_digest};
pub use mo_native_io as native_io;
pub use mo_opc as opc;
pub use mo_operation_service as operation;
pub use mo_presentation_delivery as delivery;
pub use mo_presentation_edit as edit;
pub use mo_presentation_model as model;
