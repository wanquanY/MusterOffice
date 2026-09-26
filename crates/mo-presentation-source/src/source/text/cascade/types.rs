use super::*;
use crate::source::theme::SourceThemeDefaultKind;
use mo_common::Digest;

/// This is a declaration cascade, not Office/WPS visual conformance or font binding.
pub const PROFILE: &str = "drawingml-text-cascade-draft-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextStyleOrigin {
    Object {
        object: SourceObjectRef,
        source_ordinal: u32,
    },
    Master {
        part: String,
        source_ordinal: u32,
    },
    Presentation {
        part: String,
        source_ordinal: u32,
    },
    Theme {
        part: String,
        default_kind: SourceThemeDefaultKind,
        source_ordinal: u32,
    },
    ProfileDefault {},
}
impl TextStyleOrigin {
    pub(in crate::source::text) fn at(&self, ordinal: u32) -> Self {
        let mut result = self.clone();
        match &mut result {
            Self::Object { source_ordinal, .. }
            | Self::Master { source_ordinal, .. }
            | Self::Presentation { source_ordinal, .. }
            | Self::Theme { source_ordinal, .. } => *source_ordinal = ordinal,
            Self::ProfileDefault {} => (),
        }
        result
    }
    pub(super) fn bytes(&self) -> usize {
        match self {
            Self::Object { object, .. } => object.part.len() + 64,
            Self::Master { part, .. }
            | Self::Presentation { part, .. }
            | Self::Theme { part, .. } => part.len() + 64,
            Self::ProfileDefault {} => 64,
        }
    }
}

/// Selected whole declaration. Consumers must still resolve its native semantics,
/// retained descendants, colors, theme fonts, resources, effects and permissions.
/// The reference is meaningful only with this result's immutable SourceIndex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextStyleDeclaration {
    pub element: NativeTextElement,
    pub origin: TextStyleOrigin,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ParagraphSlot {
    LineSpacing,
    SpaceBefore,
    SpaceAfter,
    BulletColor,
    BulletSize,
    BulletFont,
    Bullet,
    Tabs,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum CharacterSlot {
    Line,
    Fill,
    Effects,
    Highlight,
    UnderlineLine,
    UnderlineFill,
    Latin,
    EastAsian,
    ComplexScript,
    Symbol,
    Click,
    MouseOver,
    RightToLeft,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadedCharacterStyle {
    /// None kerning means no kerning after cascade exhaustion, not a zero threshold.
    /// Language, alternative language and bookmark can also remain unspecified.
    pub attributes: SourceTextCharacterAttributes,
    pub origins: BTreeMap<CharacterProperty, TextStyleOrigin>,
    /// Empty/mutually exclusive declarations replace a slot as a whole.
    pub declarations: BTreeMap<CharacterSlot, TextStyleDeclaration>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadedTextRun {
    pub source_ordinal: u32,
    /// Index into SourceObject.paragraphs[paragraph]. Text is not copied here.
    pub run: u32,
    pub kind: SourceRunKind,
    pub style: CascadedCharacterStyle,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadedParagraph {
    pub source_ordinal: u32,
    pub attributes: SourceTextParagraphAttributes,
    pub origins: BTreeMap<ParagraphProperty, TextStyleOrigin>,
    pub declarations: BTreeMap<ParagraphSlot, TextStyleDeclaration>,
    pub runs: Vec<CascadedTextRun>,
    /// Separate insertion/empty-paragraph style; never applied to existing runs.
    pub end_style: CascadedCharacterStyle,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadedText {
    pub profile: String,
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    /// Shape/default style fallback, independent of explicit per-script fonts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_reference: Option<TextStyleDeclaration>,
    pub paragraphs: Vec<CascadedParagraph>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextCascadeUnresolved {
    NoTextBody {},
    Placeholder {
        object: SourceObjectRef,
        matching: SourcePlaceholderMatch,
    },
    RetainedContent {
        origin: TextStyleOrigin,
    },
    AmbiguousTemplateParagraph {
        object: SourceObjectRef,
        level: i32,
    },
    FieldParagraph {
        origin: TextStyleOrigin,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum TextCascadeOutcome {
    Cascaded { text: Box<CascadedText> },
    Unresolved { reason: TextCascadeUnresolved },
}
#[derive(Debug, Clone, Copy)]
pub struct TextCascadeLimits {
    pub max_paragraphs: usize,
    pub max_runs: usize,
    pub max_steps: usize,
    /// Conservative owned lexical/reference allocation accounting, not peak RSS.
    pub max_lexical_bytes: usize,
}
impl Default for TextCascadeLimits {
    fn default() -> Self {
        Self {
            max_paragraphs: 1024,
            max_runs: 4096,
            max_steps: 1_000_000,
            max_lexical_bytes: 4 * 1024 * 1024,
        }
    }
}
