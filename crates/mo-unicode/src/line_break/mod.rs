//! Pinned UAX #14 rev.57 default opportunities; no width fitting or tailoring.
mod properties;
mod rules;
#[cfg(test)]
mod tests;
use crate::{TextBoundary, UnicodeError, UnicodeLimits};
use LineBreakClass::*;
pub use properties::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BreakKind {
    Allowed,
    Mandatory,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakOpportunity {
    pub boundary: TextBoundary,
    pub kind: BreakKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakAnalysis {
    pub profile: String,
    pub end: TextBoundary,
    /// No start-of-text opportunity; empty text has none (LB2).
    pub opportunities: Vec<LineBreakOpportunity>,
    /// Original SA positions. Default LB1 maps these to AL/CM; dictionary
    /// segmentation and target-application tailoring are separate responsibilities.
    pub complex_context_scalars: Vec<u32>,
}
#[derive(Clone, Copy)]
struct Token {
    props: LineBreakProperties,
    dotted_circle: bool,
    potential_emoji: bool,
    numeric_suffix: bool,
    ri_odd: bool,
    previous_non_space: Option<usize>,
}
impl Token {
    fn class(self) -> LineBreakClass {
        self.props.class
    }
    fn brahmic(self) -> bool {
        matches!(self.class(), Ak | As) || self.dotted_circle
    }
}
struct Scalar {
    class: LineBreakClass,
    token: usize,
    ignored: bool,
    boundary: TextBoundary,
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), UnicodeError> {
    if check() {
        Err(UnicodeError::Cancelled)
    } else {
        Ok(())
    }
}
/// Linear passes with constant-time contextual rules. Combining sequences are
/// indexed once, never repeatedly scanned backwards at each candidate boundary.
pub fn analyze_line_breaks(
    text: &str,
    limits: UnicodeLimits,
    check: &dyn Fn() -> bool,
) -> Result<LineBreakAnalysis, UnicodeError> {
    cancelled(check)?;
    if text.len() > limits.max_bytes || text.len() > u32::MAX as usize {
        return Err(UnicodeError::Limit("UTF-8 bytes"));
    }
    let mut scalars: Vec<Scalar> = Vec::new();
    let mut tokens: Vec<Token> = Vec::new();
    let mut complex_context_scalars = Vec::new();
    let mut utf16 = 0u32;
    let mut previous_non_space = None;
    for (byte, c) in text.char_indices() {
        cancelled(check)?;
        if scalars.len() >= limits.max_scalars || scalars.len() >= u32::MAX as usize {
            return Err(UnicodeError::Limit("Unicode scalars"));
        }
        let mut props = line_break_properties(c);
        let raw = props.class;
        if raw == Sa {
            complex_context_scalars.push(scalars.len() as u32);
        }
        props.class = match raw {
            Ai | Sg | Xx => Al,
            Cj => Ns,
            Sa => {
                if props.combining_mark {
                    Cm
                } else {
                    Al
                }
            }
            v => v,
        }; // LB1
        let resolved = props.class;
        let ignored = matches!(resolved, Cm | Zwj)
            && tokens
                .last()
                .is_some_and(|t| !matches!(t.class(), Bk | Cr | Lf | Nl | Sp | Zw));
        if !ignored {
            if matches!(resolved, Cm | Zwj) {
                props = line_break_properties('A');
            } // LB10, including auxiliary properties.
            let class = props.class;
            let prior = tokens.last();
            let token = Token {
                props,
                dotted_circle: c == '\u{25cc}',
                potential_emoji: props.unassigned && crate::properties(c).extended_pictographic,
                numeric_suffix: class == Nu
                    || (matches!(class, Sy | Is) && prior.is_some_and(|t| t.numeric_suffix)),
                ri_odd: class == Ri && !prior.is_some_and(|t| t.ri_odd),
                previous_non_space,
            };
            if class != Sp {
                previous_non_space = Some(tokens.len());
            }
            tokens.push(token);
        }
        scalars.push(Scalar {
            class: resolved,
            token: tokens.len() - 1,
            ignored,
            boundary: TextBoundary {
                scalar_offset: scalars.len() as u32,
                utf8_offset: byte as u32,
                utf16_offset: utf16,
            },
        });
        utf16 = utf16
            .checked_add(c.len_utf16() as u32)
            .ok_or(UnicodeError::Limit("UTF-16 coordinates"))?;
    }
    let end = TextBoundary {
        scalar_offset: scalars.len() as u32,
        utf8_offset: text.len() as u32,
        utf16_offset: utf16,
    };
    let mut opportunities = Vec::new();
    let mut physical_non_space = None;
    for i in 1..scalars.len() {
        cancelled(check)?;
        let left = &scalars[i - 1];
        let right = &scalars[i];
        if left.class != Sp {
            physical_non_space = Some(left.class);
        }
        let kind = if left.class == Bk {
            Some(BreakKind::Mandatory)
        }
        // LB4
        else if left.class == Cr && right.class == Lf {
            None
        }
        // LB5
        else if matches!(left.class, Cr | Lf | Nl) {
            Some(BreakKind::Mandatory)
        } else if matches!(right.class, Bk | Cr | Lf | Nl | Sp | Zw) {
            None
        }
        // LB6/7
        else if physical_non_space == Some(Zw) {
            Some(BreakKind::Allowed)
        }
        // LB8
        else if left.class == Zwj || right.ignored {
            None
        }
        // LB8a/9
        else {
            rules::boundary(&tokens, right.token)
        };
        if let Some(kind) = kind {
            opportunities.push(LineBreakOpportunity {
                boundary: right.boundary.clone(),
                kind,
            });
        }
    }
    if !scalars.is_empty() {
        opportunities.push(LineBreakOpportunity {
            boundary: end.clone(),
            kind: BreakKind::Mandatory,
        });
    } // LB3 after LB2
    cancelled(check)?;
    Ok(LineBreakAnalysis {
        profile: "unicode18.0.0-uax14-r57-default-v1".into(),
        end,
        opportunities,
        complex_context_scalars,
    })
}
