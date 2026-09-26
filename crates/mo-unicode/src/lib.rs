//! Fixed Unicode 18 data and extended grapheme boundaries. No host Unicode tables.
pub mod bidi;
pub mod line_break;
mod properties;
pub mod script;
pub use properties::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
pub const UNICODE_VERSION: &str = "18.0.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextBoundary {
    pub scalar_offset: u32,
    pub utf8_offset: u32,
    pub utf16_offset: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSegmentation {
    pub profile: String,
    /// Start and end boundaries; an empty text has one zero sentinel.
    pub boundaries: Vec<TextBoundary>,
    pub default_ignorables: Vec<u32>,
    pub variation_selectors: Vec<u32>,
}
#[derive(Debug, Clone, Copy)]
pub struct UnicodeLimits {
    pub max_scalars: usize,
    pub max_bytes: usize,
}
impl Default for UnicodeLimits {
    fn default() -> Self {
        Self {
            max_scalars: 65536,
            max_bytes: 262144,
        }
    }
}
#[derive(Debug, Error)]
pub enum UnicodeError {
    #[error("Unicode text budget exceeded: {0}")]
    Limit(&'static str),
    #[error("Unicode text analysis cancelled")]
    Cancelled,
}
#[derive(Default)]
struct State {
    previous: Option<GraphemeBreak>,
    ri_odd: bool,
    ep_extend: bool,
    zwj_after_ep: bool,
    incb_linker_extend: bool,
}
impl State {
    fn boundary(&self, current: Properties) -> bool {
        use GraphemeBreak::*;
        let Some(previous) = self.previous else {
            return true;
        };
        let next = current.grapheme_break;
        // Rules are ordered. GB4/5 must win over Extend/Prepend rules.
        if previous == Cr && next == Lf {
            return false;
        } // GB3
        if matches!(previous, Control | Cr | Lf) || matches!(next, Control | Cr | Lf) {
            return true;
        } // GB4/5
        if previous == L && matches!(next, L | V | Lv | Lvt) {
            return false;
        } // GB6
        if matches!(previous, Lv | V) && matches!(next, V | T) {
            return false;
        } // GB7
        if matches!(previous, Lvt | T) && next == T {
            return false;
        } // GB8
        if matches!(next, Extend | Zwj | SpacingMark) || previous == Prepend {
            return false;
        } // GB9/9a/9b
        if self.incb_linker_extend && current.indic_conjunct == IndicConjunct::Consonant {
            return false;
        } // Unicode 18 GB9c
        if self.zwj_after_ep && current.extended_pictographic {
            return false;
        } // GB11
        if self.ri_odd && next == RegionalIndicator {
            return false;
        } // GB12/13
        true // GB999
    }
    fn consume(&mut self, current: Properties) {
        use GraphemeBreak::*;
        self.ri_odd = if current.grapheme_break == RegionalIndicator {
            !self.ri_odd
        } else {
            false
        };
        self.zwj_after_ep = current.grapheme_break == Zwj && self.ep_extend;
        self.ep_extend =
            current.extended_pictographic || (current.grapheme_break == Extend && self.ep_extend);
        self.incb_linker_extend = match current.indic_conjunct {
            IndicConjunct::Linker => true,
            IndicConjunct::Extend => self.incb_linker_extend,
            _ => false,
        };
        self.previous = Some(current.grapheme_break);
    }
}
/// Forward UAX #29 rev.49 extended-grapheme state machine. Constant state and
/// bounded table lookups; no backwards rescanning of long combining sequences.
pub fn segment(
    text: &str,
    limits: UnicodeLimits,
    check: &dyn Fn() -> bool,
) -> Result<TextSegmentation, UnicodeError> {
    if check() {
        return Err(UnicodeError::Cancelled);
    }
    if text.len() > limits.max_bytes || text.len() > u32::MAX as usize {
        return Err(UnicodeError::Limit("UTF-8 bytes"));
    }
    let mut result = TextSegmentation {
        profile: "unicode18.0.0-uax29-r49-extended-v1".into(),
        boundaries: vec![],
        default_ignorables: vec![],
        variation_selectors: vec![],
    };
    let mut state = State::default();
    let mut scalar = 0u32;
    let mut utf16 = 0u32;
    for (byte, c) in text.char_indices() {
        if check() {
            return Err(UnicodeError::Cancelled);
        }
        if scalar as usize >= limits.max_scalars {
            return Err(UnicodeError::Limit("Unicode scalars"));
        }
        let props = properties(c);
        if state.boundary(props) {
            result.boundaries.push(TextBoundary {
                scalar_offset: scalar,
                utf8_offset: byte as u32,
                utf16_offset: utf16,
            });
        }
        if props.default_ignorable {
            result.default_ignorables.push(scalar);
        }
        if props.variation_selector {
            result.variation_selectors.push(scalar);
        }
        state.consume(props);
        scalar += 1;
        utf16 = utf16
            .checked_add(c.len_utf16() as u32)
            .ok_or(UnicodeError::Limit("UTF-16 coordinates"))?;
    }
    result.boundaries.push(TextBoundary {
        scalar_offset: scalar,
        utf8_offset: text.len() as u32,
        utf16_offset: utf16,
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn offsets(s: &str) -> Vec<u32> {
        segment(s, UnicodeLimits::default(), &|| false)
            .unwrap()
            .boundaries
            .iter()
            .map(|b| b.scalar_offset)
            .collect()
    }
    #[test]
    fn unicode18_linker_rule_does_not_require_a_leading_consonant() {
        assert_eq!(offsets("\u{94d}\u{915}"), vec![0, 2]);
    }
    #[test]
    fn emoji_flags_and_combining_marks_keep_scalar_and_utf16_coordinates_distinct() {
        assert_eq!(offsets("A\u{301}👩🏽‍💻🇨🇳🇺🇸"), vec![0, 2, 6, 8, 10]);
        let r = segment("A😀", UnicodeLimits::default(), &|| false).unwrap();
        assert_eq!(
            r.boundaries.last().unwrap(),
            &TextBoundary {
                scalar_offset: 2,
                utf8_offset: 5,
                utf16_offset: 3
            }
        );
    }
    #[test]
    fn controls_override_prepend_and_extenders() {
        assert_eq!(offsets("\u{600}\r\n\u{301}"), vec![0, 1, 3, 4]);
        assert_eq!(offsets(""), vec![0]);
    }
    #[test]
    fn limits_and_midstream_cancellation_fail_without_partial_output() {
        assert!(matches!(
            segment(
                "abc",
                UnicodeLimits {
                    max_scalars: 2,
                    max_bytes: 9
                },
                &|| false
            ),
            Err(UnicodeError::Limit(_))
        ));
        let n = std::cell::Cell::new(0);
        assert!(matches!(
            segment("abc", UnicodeLimits::default(), &|| {
                n.set(n.get() + 1);
                n.get() == 3
            }),
            Err(UnicodeError::Cancelled)
        ));
    }
}
