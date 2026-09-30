use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Explicit display symbols, independent of the machine's regional settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NumberSymbols {
    pub decimal_separator: String,
    pub group_separator: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum NumberFragment {
    Text {
        value: String,
    },
    /// Measured advance of this glyph, not an ASCII-space substitution.
    Reserve {
        glyph: char,
    },
    /// The layout engine repeats this glyph to fill remaining available width.
    Fill {
        glyph: char,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NumberFormatColor {
    Black,
    Blue,
    Cyan,
    Green,
    Magenta,
    Red,
    White,
    Yellow,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NumberDisplay {
    pub section: u8,
    pub color: Option<NumberFormatColor>,
    pub fragments: Vec<NumberFragment>,
    /// The displayed numeric magnitude differs from the exact input.
    pub rounded: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NumberFormatReason {
    InvalidSyntax,
    ConditionalSection,
    LocaleDirective,
    IndexedColor,
    DateTime,
    Fraction,
    TextPlaceholder,
    GeneralNeedsLayout,
    UnsupportedToken,
    GroupedEmbeddedLiteral,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NumberFormatIssue {
    /// UTF-8 byte offset in the unmodified source format code.
    pub offset: u32,
    pub reason: NumberFormatReason,
}
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("unresolved numeric format at byte {}: {:?}", .0.offset, .0.reason)]
    Unresolved(NumberFormatIssue),
    #[error("number format budget: {0}")]
    Limit(&'static str),
    #[error("number format cancelled")]
    Cancelled,
    #[error("invalid number display symbols")]
    Symbols,
    #[error("invalid exact number fraction")]
    Ratio,
}
#[derive(Debug, Clone, Copy)]
pub struct FormatLimits {
    pub max_code_bytes: usize,
    pub max_formats: usize,
    pub max_total_code_bytes: usize,
    pub max_numeric_digits: usize,
    pub max_total_numeric_digits: usize,
    pub max_fraction_digits: usize,
    pub max_output_bytes: usize,
}
impl Default for FormatLimits {
    fn default() -> Self {
        Self {
            max_code_bytes: 4096,
            max_formats: 128,
            max_total_code_bytes: 65536,
            max_numeric_digits: 16384,
            max_total_numeric_digits: 1024 * 1024,
            max_fraction_digits: 256,
            max_output_bytes: 4 * 1024 * 1024,
        }
    }
}
