use mo_common::*;
use mo_presentation_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSelection {
    pub anchor: TextAnchor,
    pub focus: TextAnchor,
}

/// Omitted fields keep their declarations; `inherit` explicitly resets one
/// field. Applying bold must not resolve or overwrite font/theme inheritance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterStylePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Inherited<FontId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Inherited<Emu>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Inherited<Color>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<Inherited<bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<Inherited<bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<Inherited<bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Inherited<String>>,
}

impl CharacterStylePatch {
    pub(crate) fn apply(&self, style: &mut CharacterStyle) {
        macro_rules! fields {
            ($($field:ident),+) => {$(
                if let Some(value) = &self.$field { style.$field = value.clone(); }
            )+};
        }
        fields!(font, size, color, bold, italic, underline, language);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TextEditAction {
    /// CRLF and CR become paragraph breaks. Surviving runs keep their style;
    /// inserted text inherits the declaration at the start caret's affinity.
    Replace {
        selection: TextSelection,
        text: String,
    },
    SetCharacterStyle {
        selection: TextSelection,
        patch: CharacterStylePatch,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextEditCommand {
    pub document_id: DocumentId,
    pub request_id: RequestId,
    pub base_revision: Digest,
    pub operation_id: OperationId,
    pub object: ObjectId,
    pub action: TextEditAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextEditCandidate {
    pub snapshot: crate::SnapshotRecord,
    pub receipt: crate::TransactionReceipt,
    pub transaction: crate::Transaction,
    pub command_digest: Digest,
    pub selection: TextSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_change: Option<TextRangeChange>,
}

/// Ordered before/after ranges, including paragraph identities. All unaffected
/// paragraphs keep their anchors. Reversal swaps the two ranges and lists.
/// Coordinates are exact scalar offsets; joining text can turn a former
/// boundary into the interior of a grapheme. A display caret must resolve the
/// mapped offset against the resulting paragraph's grapheme boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextRangeChange {
    pub object: ObjectId,
    pub before: TextSelection,
    pub after: TextSelection,
    pub before_paragraphs: Vec<ParagraphId>,
    pub after_paragraphs: Vec<ParagraphId>,
}

impl TextRangeChange {
    pub fn reversed(&self) -> Self {
        Self {
            object: self.object.clone(),
            before: self.after.clone(),
            after: self.before.clone(),
            before_paragraphs: self.after_paragraphs.clone(),
            after_paragraphs: self.before_paragraphs.clone(),
        }
    }

    pub fn map(&self, anchor: &TextAnchor) -> Result<TextAnchor, crate::EditError> {
        let start = &self.before.anchor;
        let end = &self.before.focus;
        if !self.before_paragraphs.contains(&anchor.paragraph) {
            return Ok(anchor.clone());
        }
        if anchor.paragraph == start.paragraph
            && (anchor.scalar_offset < start.scalar_offset
                || (anchor.scalar_offset == start.scalar_offset
                    && anchor.affinity == Affinity::Before))
        {
            return Ok(anchor.clone());
        }
        let deleted = start.paragraph != end.paragraph || start.scalar_offset != end.scalar_offset;
        if anchor.paragraph == end.paragraph
            && (anchor.scalar_offset > end.scalar_offset
                || (anchor.scalar_offset == end.scalar_offset && deleted))
        {
            return Ok(TextAnchor {
                paragraph: self.after.focus.paragraph.clone(),
                scalar_offset: self
                    .after
                    .focus
                    .scalar_offset
                    .checked_add(anchor.scalar_offset - end.scalar_offset)
                    .ok_or_else(|| crate::EditError::input("text anchor overflow"))?,
                affinity: anchor.affinity,
            });
        }
        let target = if anchor.affinity == Affinity::Before {
            &self.after.anchor
        } else {
            &self.after.focus
        };
        Ok(TextAnchor {
            affinity: anchor.affinity,
            ..target.clone()
        })
    }
}
