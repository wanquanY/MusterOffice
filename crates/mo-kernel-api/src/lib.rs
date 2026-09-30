mod pptx_import;
pub use pptx_import::*;
mod delivery;
mod delivery_playback;
mod image_decode;
pub use delivery::*;
pub use delivery_playback::*;
mod playback;
pub use playback::*;
mod playback_owner;
mod playback_session;
mod pptx_playback_session;
pub use playback_session::*;
pub use pptx_playback_session::*;
mod timeline;
pub use timeline::*;
mod pptx_timing;
pub use image_decode::*;
pub use pptx_timing::*;
// Shared native/WASM computation boundary. No persistence, filesystem or authority.
mod flow;
pub use flow::*;
mod geometry;
pub use geometry::*;
mod paragraph;
pub use paragraph::*;
mod font;
pub use font::*;
mod text;
pub use text::*;
mod cascade;
pub use cascade::*;
mod unicode;
pub use unicode::*;
mod bidi;
pub use bidi::*;
mod line_break;
pub use line_break::*;
mod scene;
pub use scene::*;
mod raster;
pub use raster::*;
mod image_raster;
pub use image_raster::*;
mod image_scene;
pub use image_scene::*;
mod render;
pub use render::*;
mod page;
pub use page::*;
mod placement;
pub use placement::*;
mod outlines;
pub use outlines::*;
mod metrics;
pub use metrics::*;
mod chart_geometry;
mod chart_sectors;
pub use chart_geometry::*;
mod package;
mod pptx;
mod pptx_chart_geometry;
mod pptx_chart_labels;
mod pptx_chart_paints;
mod pptx_chart_plot;
mod pptx_charts;
pub use chart_sectors::*;
pub use pptx_chart_geometry::*;
pub use pptx_chart_labels::*;
pub use pptx_chart_paints::*;
pub use pptx_chart_plot::*;
mod pptx_color;
mod pptx_fill;
mod pptx_fill_color;
mod pptx_geometry;
pub use pptx_charts::*;
mod pptx_images;
mod pptx_line;
mod pptx_line_color;
mod pptx_page;
mod pptx_paths;
mod pptx_placement;
mod pptx_playback;
mod pptx_radial;
mod pptx_resource_page;
pub use pptx_playback::*;
mod pptx_source;
mod pptx_table_borders;
pub use pptx_table_borders::*;
mod pptx_text_body;
mod pptx_text_page;
use mo_common::{Digest, from_json_str};
pub use mo_pptx::source::{SourceLimits, SourceTextEdits, SourceTransformEdits};
use mo_presentation_edit::{
    EditError, Snapshot, SnapshotRecord, Transaction, TransactionReceipt, prepare,
};
use mo_presentation_model::{Document, ValidationLimits, ValidationReport, validate};
pub use package::*;
pub use pptx::*;
pub use pptx_color::*;
pub use pptx_fill::*;
pub use pptx_fill_color::*;
pub use pptx_geometry::*;
pub use pptx_images::*;
pub use pptx_line::*;
pub use pptx_line_color::*;
pub use pptx_page::*;
pub use pptx_paths::*;
pub use pptx_placement::*;
pub use pptx_radial::*;
pub use pptx_resource_page::*;
pub use pptx_source::*;
pub use pptx_text_body::*;
pub use pptx_text_page::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum KernelRequest {
    Validate {
        document: Document,
    },
    Initialize {
        document: Document,
    },
    Prepare {
        snapshot: SnapshotRecord,
        transaction: Transaction,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum KernelResponse {
    Validated {
        report: ValidationReport,
    },
    Initialized {
        snapshot: SnapshotRecord,
    },
    Prepared {
        snapshot: SnapshotRecord,
        receipt: Box<TransactionReceipt>,
    },
    Error {
        error: KernelError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Cancelled,
    InputInvalid,
    RevisionConflict,
    ReferenceConflict,
    RequestIdReused,
    LimitExceeded,
    InternalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KernelError {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operation_ids: Vec<mo_common::OperationId>,
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ValidationReport>,
}

fn error(code: ErrorCode, message: String) -> KernelResponse {
    KernelResponse::Error {
        error: KernelError {
            operation_ids: Vec::new(),
            code,
            message,
            current_revision: None,
            report: None,
        },
    }
}

fn edit_error(error: EditError) -> KernelResponse {
    use mo_presentation_edit::EditDiagnosticCode as Code;
    let diagnostic = error.diagnostic();
    let code = match diagnostic.code {
        Code::Cancelled => ErrorCode::Cancelled,
        Code::InputInvalid => ErrorCode::InputInvalid,
        Code::RevisionConflict => ErrorCode::RevisionConflict,
        Code::ReferenceConflict => ErrorCode::ReferenceConflict,
        Code::RequestIdReused => ErrorCode::RequestIdReused,
    };
    KernelResponse::Error {
        error: KernelError {
            code,
            message: diagnostic.message,
            operation_ids: diagnostic.operation_ids,
            current_revision: diagnostic.current_revision,
            report: diagnostic.report,
        },
    }
}

pub fn dispatch(request: KernelRequest, limits: ValidationLimits) -> KernelResponse {
    match request {
        KernelRequest::Validate { document } => KernelResponse::Validated {
            report: validate(&document, limits),
        },
        KernelRequest::Initialize { document } => match Snapshot::new(document, limits) {
            Ok(snapshot) => KernelResponse::Initialized {
                snapshot: snapshot.into_record(),
            },
            Err(error) => edit_error(error),
        },
        KernelRequest::Prepare {
            snapshot,
            transaction,
        } => {
            match Snapshot::restore(snapshot, limits)
                .and_then(|base| prepare(&base, &transaction, limits))
            {
                Ok(prepared) => KernelResponse::Prepared {
                    snapshot: prepared.snapshot.into_record(),
                    receipt: Box::new(prepared.receipt),
                },
                Err(error) => edit_error(error),
            }
        }
    }
}

/// Development wire boundary used unchanged by CLI and WASM. Not a product MCP envelope.
pub fn dispatch_json(input: &str) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        error(
            ErrorCode::LimitExceeded,
            "request exceeds 32 MiB byte budget".into(),
        )
    } else {
        match from_json_str::<KernelRequest>(input) {
            Ok(request) => dispatch(request, ValidationLimits::default()),
            Err(reason) => error(ErrorCode::InputInvalid, reason.to_string()),
        }
    };
    serde_json::to_string(&response)
        .expect("typed kernel response contains only JSON-serializable values")
}
