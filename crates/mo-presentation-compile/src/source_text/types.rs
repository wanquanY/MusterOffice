use super::*;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_presentation_source::source::{
    SourceRunKind,
    text::{cascade::*, fonts::*},
};
use mo_text::{
    flow::OverflowPolicy,
    geometry::{GeometryStyle, LineSpacing},
    itemize::*,
    manifest::*,
};
use mo_unicode::bidi::ParagraphDirection;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum SourceTextPreparation {
    Prepared { text: PreparedSourceText },
    Unresolved { issue: SourceTextIssue },
}
#[derive(Debug, thiserror::Error)]
pub enum SourceTextError {
    #[error(transparent)]
    FontSelection(Box<SourceFontSelectionFailure>),
    #[error(transparent)]
    Source(#[from] mo_presentation_source::PptxError),
    #[error(transparent)]
    Text(#[from] mo_text::TextError),
    #[error("source text cancelled")]
    Cancelled,
    #[error("source text limit exceeded: {0}")]
    Limit(&'static str),
    #[error("invalid source text binding: {0}")]
    Invalid(&'static str),
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SourceTextIssue {
    Cascade {
        reason: TextCascadeUnresolved,
    },
    Field {
        paragraph: u32,
        run: u32,
        source_ordinal: u32,
    },
    CharacterProperty {
        paragraph: u32,
        run: Option<u32>,
        property: CharacterProperty,
    },
    DirectionOverride {
        paragraph: u32,
        run: Option<u32>,
    },
    SymbolFont {
        paragraph: u32,
        run: Option<u32>,
    },
    ParagraphControl {
        paragraph: u32,
        run: u32,
    },
    Itemization {
        paragraph: u32,
        notice: ItemizationNotice,
    },
    Script {
        paragraph: u32,
        start: u32,
        end: u32,
        script: String,
    },
    Typeface {
        paragraph: u32,
        run: Option<u32>,
        slot: NativeFontSlot,
        reason: TypefaceUnresolved,
    },
    GraphemeStyleConflict {
        paragraph: u32,
        boundary: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceScalarRange {
    pub start: u32,
    pub end: u32,
    pub run: u32,
    pub source_ordinal: u32,
    pub kind: SourceRunKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFontBinding {
    pub run: Option<u32>,
    pub script: String,
    pub slot: NativeFontSlot,
    pub theme_script: Option<String>,
    pub font: NativeTypeface,
    pub style: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, thiserror::Error)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[error("font selection at {object:?}, paragraph {paragraph}: {selection}")]
pub struct SourceFontSelectionFailure {
    pub source_sha256: Digest,
    pub object: mo_presentation_source::source::SourceObjectRef,
    pub paragraph: u32,
    pub source_ordinal: u32,
    pub selection: Box<FontSelectionFailure>,
    /// Every source binding sharing the failed computation style, including
    /// insertion style (run=None), with native declaration/theme provenance.
    pub uses: Vec<SourceFontBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFontSpan {
    pub start: u32,
    pub end: u32,
    pub binding: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceParagraphPlan {
    pub profile: String,
    pub source_ordinal: u32,
    pub sources: Vec<SourceScalarRange>,
    #[serde(flatten)]
    pub computation: ParagraphComputationPlan,
}
impl std::ops::Deref for SourceParagraphPlan {
    type Target = ParagraphComputationPlan;
    fn deref(&self) -> &Self::Target {
        &self.computation
    }
}
/// Shared immutable computation input. Native source bindings remain in each
/// domain's containing plan; generated chart strings never acquire XML run ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphComputationPlan {
    /// Source a:br is represented by U+2028. Existing a:t is never normalized.
    pub text: String,
    pub direction: ParagraphDirection,
    pub fonts: Vec<SourceFontBinding>,
    pub font_spans: Vec<SourceFontSpan>,
    pub spans: Vec<StyleSpan>,
    pub styles: Vec<ManifestTextStyle>,
    pub geometry: Vec<GeometryStyle>,
    /// Maximum native baseline conversion uncertainty, including styles that
    /// coalesced after Q32 conversion. Independent of the wire representation.
    pub baseline_conversion_error: Fixed,
    /// Maximum source tracking conversion uncertainty before style coalescing.
    pub tracking_conversion_error: Fixed,
    /// Insertion/empty-line style, never used to overwrite existing run styles.
    pub end_style: u32,
}
impl ParagraphComputationPlan {
    pub(crate) fn input(&self) -> ManifestParagraphInput<'_> {
        ManifestParagraphInput {
            text: &self.text,
            direction: self.direction,
            spans: &self.spans,
            styles: &self.styles,
        }
    }
}
#[derive(Debug, Clone)]
pub struct SourceGlyphFlow {
    pub width: Emu,
    pub spacing: LineSpacing,
    pub overflow: OverflowPolicy,
    pub bounds_tolerance: Fixed,
}
#[derive(Debug, Clone, Copy)]
pub struct SourceTextLimits {
    pub cascade: TextCascadeLimits,
    pub typeface: TypefaceLimits,
    pub max_text_bytes: usize,
    /// Conservative owned plan allocation accounting; not a measured RSS cap.
    pub max_plan_bytes: usize,
    pub max_font_bindings: usize,
}
impl Default for SourceTextLimits {
    fn default() -> Self {
        Self {
            cascade: TextCascadeLimits::default(),
            typeface: TypefaceLimits::default(),
            max_text_bytes: 4 * 1024 * 1024,
            max_plan_bytes: 32 * 1024 * 1024,
            max_font_bindings: 4096,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceParagraphComputation<T> {
    pub profile: String,
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    pub paragraph: u32,
    pub source_ordinal: u32,
    pub computation: T,
}
