use super::super::*;
use crate::source::{SourceObjectRef, SourcePlaceholderMatch, theme::SourceThemeDefaultKind};
use mo_common::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TextBodyProfile {
    #[serde(rename = "drawingml-body-inheritance-draft-v1")]
    DrawingmlDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextBodyQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub objects: Vec<u32>,
    pub profile: TextBodyProfile,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextBodyOrigin {
    Cell {
        object: SourceObjectRef,
        cell: crate::source::table::SourceCellAddress,
        source_ordinal: u32,
    },
    CellDefault {
        object: SourceObjectRef,
        cell: crate::source::table::SourceCellAddress,
    },
    Object {
        object: SourceObjectRef,
        source_ordinal: u32,
    },
    Theme {
        part: String,
        default_kind: Option<SourceThemeDefaultKind>,
        source_ordinal: u32,
    },
    ProfileDefault {},
}
impl TextBodyOrigin {
    pub(super) fn at(&self, ordinal: u32) -> Self {
        let mut r = self.clone();
        match &mut r {
            Self::Object { source_ordinal, .. }
            | Self::Theme { source_ordinal, .. }
            | Self::Cell { source_ordinal, .. } => *source_ordinal = ordinal,
            _ => (),
        }
        r
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveTextAutofit {
    None {
        declared_by: TextBodyOrigin,
    },
    Shape {
        declared_by: TextBodyOrigin,
    },
    Normal {
        declared_by: TextBodyOrigin,
        font_scale: NativePercentage,
        line_spacing_reduction: NativePercentage,
        /// Defaulted within the chosen normAutofit element, not inherited from a different choice.
        font_scale_defaulted: bool,
        line_spacing_reduction_defaulted: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveTextBody {
    /// All 19 fields are populated after the explicit profile defaults are applied.
    pub attributes: SourceTextBodyAttributes,
    pub origins: BTreeMap<TextBodyProperty, TextBodyOrigin>,
    pub autofit: EffectiveTextAutofit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextBodyUnresolved {
    NoTextBody {},
    Placeholder {
        object: SourceObjectRef,
        matching: SourcePlaceholderMatch,
    },
    RetainedContent {
        origin: TextBodyOrigin,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum TextBodyOutcome {
    Resolved { body: Box<EffectiveTextBody> },
    Unresolved { reason: TextBodyUnresolved },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextBodyResult {
    pub native_id: u32,
    pub outcome: TextBodyOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextBodies {
    pub source_sha256: Digest,
    pub surface: String,
    pub profile: TextBodyProfile,
    pub objects: Vec<SourceTextBodyResult>,
}
#[derive(Debug, Clone, Copy)]
pub struct TextBodyLimits {
    pub max_queries: usize,
    pub max_steps: usize,
    pub max_lexical_bytes: usize,
}
impl Default for TextBodyLimits {
    fn default() -> Self {
        Self {
            max_queries: 256,
            max_steps: 1_000_000,
            max_lexical_bytes: 4 * 1024 * 1024,
        }
    }
}

// One field list supplies both typed provenance keys and property-by-property merge.
macro_rules! fields {
    ($($field:ident => $variant:ident),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub enum TextBodyProperty { $($variant),* }
        #[allow(clippy::clone_on_copy)] // The same field list includes owned native lexicals.
        pub(super) fn inherit(target: &mut SourceTextBodyAttributes, from: &SourceTextBodyAttributes,
            origins: &mut BTreeMap<TextBodyProperty, TextBodyOrigin>, origin: &TextBodyOrigin) {
            $(if target.$field.is_none() && from.$field.is_some() {
                target.$field = from.$field.clone();
                origins.insert(TextBodyProperty::$variant, origin.clone());
            })*
        }
    }
}
fields! { rotation => Rotation, paragraph_spacing => ParagraphSpacing,
vertical_overflow => VerticalOverflow, horizontal_overflow => HorizontalOverflow,
vertical => Vertical, wrap => Wrap, left_inset => LeftInset, top_inset => TopInset,
right_inset => RightInset, bottom_inset => BottomInset, columns => Columns,
column_spacing => ColumnSpacing, right_to_left_columns => RightToLeftColumns,
from_word_art => FromWordArt, anchor => Anchor, center_anchor => CenterAnchor,
force_antialiasing => ForceAntialiasing, upright => Upright, compatible_line_spacing => CompatibleLineSpacing }

pub(super) fn defaults() -> SourceTextBodyAttributes {
    let coordinate =
        |s: &str| Some(NativeCoordinate::try_from(s.to_owned()).expect("profile coordinate"));
    SourceTextBodyAttributes {
        rotation: Some(0),
        paragraph_spacing: Some(false),
        vertical_overflow: Some(NativeTextVerticalOverflow::Overflow),
        horizontal_overflow: Some(NativeTextHorizontalOverflow::Overflow),
        vertical: Some(NativeTextVertical::Horz),
        wrap: Some(NativeTextWrap::Square),
        left_inset: coordinate("91440"),
        top_inset: coordinate("45720"),
        right_inset: coordinate("91440"),
        bottom_inset: coordinate("45720"),
        columns: Some(1),
        column_spacing: coordinate("0"),
        right_to_left_columns: Some(false),
        from_word_art: Some(false),
        anchor: Some(NativeTextAnchor::T),
        center_anchor: Some(false),
        force_antialiasing: Some(false),
        upright: Some(false),
        compatible_line_spacing: Some(false),
    }
}
