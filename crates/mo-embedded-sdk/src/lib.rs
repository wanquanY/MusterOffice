//! Caller-owned presentation computation with explicit input and output.
//! No account, permissions, durable task, database or product UI is required.
//! Native graphics use a configured worker and caller-provided temporary I/O;
//! the product owns final saving/publication and any durable lifecycle.
mod execute;
mod inputs;
mod presentation;
pub use execute::{Execution, execute};
pub use inputs::Inputs;
pub use mo_common as common;
pub use mo_native_export::{NativeExportCandidate, NativeExporter, executable_digest};
pub use mo_native_io as native_io;
pub use mo_native_render::playback;
pub use mo_opc as opc;
pub use mo_presentation_delivery as delivery;
pub use mo_presentation_edit as edit;
pub use mo_presentation_model as model;
pub use mo_presentation_operations as operation;
pub use mo_presentation_template as template;
pub use mo_timeline as timeline;
pub use presentation::{ExportOptions, PlaybackOptions, Presentation};
