//! Frozen legacy wire grammar. New pure computations must not implicitly gain
//! a persistence or authorization path merely because Rust types are shared.
use crate::{AssetBinding, DocumentAction, ExportSettings};
use mo_common::{Digest, DocumentId};
use mo_presentation_edit::OperationEntry;
use mo_presentation_model::Document;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "DocumentAction")]
pub(crate) enum LegacyAction {
    Import {
        document_id: DocumentId,
        source: AssetBinding,
    },
    Create {
        document: Box<Document>,
    },
    Apply {
        document_id: DocumentId,
        base_revision: Digest,
        operations: Vec<OperationEntry>,
    },
    Export {
        document_id: DocumentId,
        base_revision: Digest,
        settings: Box<ExportSettings>,
    },
}
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(value: D) -> Result<DocumentAction, D::Error> {
    Ok(match LegacyAction::deserialize(value)? {
        LegacyAction::Import {
            document_id,
            source,
        } => DocumentAction::Import {
            document_id,
            source,
        },
        LegacyAction::Create { document } => DocumentAction::Create { document },
        LegacyAction::Apply {
            document_id,
            base_revision,
            operations,
        } => DocumentAction::Apply {
            document_id,
            base_revision,
            operations,
        },
        LegacyAction::Export {
            document_id,
            base_revision,
            settings,
        } => DocumentAction::Export {
            document_id,
            base_revision,
            settings,
        },
    })
}
