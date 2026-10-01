use super::*;
use mo_charts::number_format::{NumberFormatColor, NumberFormatIssue};
use mo_presentation_source::{PptxError, source::text::SourceTextParagraphAttributes};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ChartLabelTextPreparation {
    Prepared {
        text: PreparedChartLabelText,
    },
    Unresolved {
        target: ChartLabelTarget,
        issue: ChartTextIssue,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChartTextIssue {
    MissingTextProperties {},
    MissingCharacterSize {
        paragraph: u32,
        segment: Option<u32>,
    },
    UnresolvedFlags {
        flags: Vec<ChartLabelFlag>,
    },
    TextCascade {
        reason: TextCascadeUnresolved,
    },
    MissingNumberDisplay {},
    MissingNumberFormat {
        component: u32,
    },
    NumberFormat {
        component: u32,
        issue: NumberFormatIssue,
    },
    NumberSpacing {
        component: u32,
        fragment: u32,
    },
    CustomStringReference {},
    MissingSeparator {},
    Computation {
        issue: SourceTextIssue,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum ChartTextError {
    #[error(transparent)]
    Source(#[from] PptxError),
    #[error(transparent)]
    Labels(#[from] ChartLabelError),
    #[error(transparent)]
    Preparation(#[from] SourceTextError),
    #[error(transparent)]
    Text(#[from] mo_text::TextError),
    #[error("chart label font selection: {0:?}")]
    FontSelection(Box<ChartTextComputation<ChartFontSelectionFailure>>),
    #[error("chart text cancelled")]
    Cancelled,
    #[error("invalid chart text binding: {0}")]
    Invalid(&'static str),
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ChartLabelTextLimits {
    pub labels: SourceChartLabelLimits,
    pub text: SourceTextLimits,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelText {
    pub target: ChartLabelTarget,
    /// Deleted or explicitly empty labels have no paragraphs.
    pub paragraphs: Vec<ChartTextParagraph>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartTextParagraph {
    /// Actual chart paragraph owning rich text or generated-content defaults.
    pub source_ordinal: u32,
    pub attributes: SourceTextParagraphAttributes,
    pub characters: Vec<CascadedCharacterStyle>,
    pub end_style: CascadedCharacterStyle,
    pub sources: Vec<ChartTextRange>,
    pub computation: ParagraphComputationPlan,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartTextRange {
    pub start: u32,
    pub end: u32,
    /// Index into this paragraph's character styles; font-binding run means the
    /// segment index in this array, not a native XML run for generated content.
    pub character: u32,
    pub origin: ChartTextOrigin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChartTextOrigin {
    NativeRun {
        source_ordinal: u32,
        text_source_ordinal: Option<u32>,
        run: u32,
    },
    Component {
        component: u32,
        fragment: Option<u32>,
        color: Option<NumberFormatColor>,
    },
    Separator {
        before_component: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartTextComputation<T> {
    pub profile: String,
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    pub chart_part: String,
    pub chart_sha256: Digest,
    pub target: ChartLabelTarget,
    pub paragraph: u32,
    pub computation: T,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartFontSelectionFailure {
    pub selection: Box<FontSelectionFailure>,
    pub uses: Vec<SourceFontBinding>,
}
