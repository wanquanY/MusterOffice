use crate::{PptxPageFailure, ShapeFailure, pptx_page};
use mo_pptx::source::{
    color::ColorUnresolved,
    text::{
        cascade::{CharacterProperty, TextStyleDeclaration},
        paint::{TextPaintError, TextPaintLocation},
    },
};
use mo_presentation_compile::{
    source_frame::{SourceFrameError, SourceFrameIssue},
    source_page::SourcePageError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "stage",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PptxTextPageFailure {
    Request {
        error: PptxPageFailure,
    },
    Source {
        error: PptxPageFailure,
    },
    Fonts {
        error: ShapeFailure,
    },
    Page {
        error: PptxPageFailure,
        #[serde(skip_serializing_if = "Option::is_none")]
        paint_location: Option<TextPaintLocation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<Box<PptxTextPageIssue>>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PptxTextPageIssue {
    DecorationMetric {
        reason: mo_presentation_compile::source_text_page::TextDecorationIssue,
    },
    FontSelection {
        failure: mo_presentation_compile::source_text::SourceFontSelectionFailure,
    },
    Frame {
        reason: SourceFrameIssue,
    },
    PaintProperty {
        property: CharacterProperty,
    },
    PaintDeclaration {
        declaration: TextStyleDeclaration,
    },
    Color {
        declaration: TextStyleDeclaration,
        reason: ColorUnresolved,
    },
    MissingPaint {},
    GlyphPaintConflict {
        paragraph: u32,
        start: u32,
        end: u32,
    },
}
pub(crate) fn page_failure(e: SourcePageError) -> PptxTextPageFailure {
    let mut leaf = &e;
    while let SourcePageError::AtObject { error, .. } = leaf {
        leaf = error;
    }
    let detail = match leaf {
        SourcePageError::TextDecoration(reason) => Some(PptxTextPageIssue::DecorationMetric {
            reason: (**reason).clone(),
        }),
        SourcePageError::Text(SourceFrameError::SourceText(
            mo_presentation_compile::source_text::SourceTextError::FontSelection(failure),
        )) => Some(PptxTextPageIssue::FontSelection {
            failure: (**failure).clone(),
        }),
        SourcePageError::Text(SourceFrameError::Mapping(reason)) => {
            Some(PptxTextPageIssue::Frame {
                reason: (**reason).clone(),
            })
        }
        SourcePageError::TextPaint(e) => paint_detail(e),
        SourcePageError::GlyphPaintConflict {
            paragraph,
            start,
            end,
        } => Some(PptxTextPageIssue::GlyphPaintConflict {
            paragraph: *paragraph,
            start: *start,
            end: *end,
        }),
        _ => None,
    };
    let mut paint_location = None;
    if let SourcePageError::TextDecoration(reason) = leaf {
        paint_location = Some(TextPaintLocation {
            paragraph: reason.paragraph,
            run: reason.run,
            source_ordinal: reason.source_ordinal,
        });
    }
    if let SourcePageError::TextPaint(first) = leaf {
        let mut paint = first;
        while let TextPaintError::AtRun { location, error } = paint {
            paint_location.get_or_insert_with(|| location.clone());
            paint = error;
        }
    }
    PptxTextPageFailure::Page {
        error: pptx_page::failure(e),
        paint_location,
        detail: detail.map(Box::new),
    }
}

fn paint_detail(e: &TextPaintError) -> Option<PptxTextPageIssue> {
    match e {
        TextPaintError::AtRun { error, .. } => paint_detail(error),
        TextPaintError::Property(property) => Some(PptxTextPageIssue::PaintProperty {
            property: *property,
        }),
        TextPaintError::Declaration(declaration) => Some(PptxTextPageIssue::PaintDeclaration {
            declaration: (**declaration).clone(),
        }),
        TextPaintError::Color {
            declaration,
            reason,
        } => Some(PptxTextPageIssue::Color {
            declaration: (**declaration).clone(),
            reason: reason.clone(),
        }),
        TextPaintError::Missing => Some(PptxTextPageIssue::MissingPaint {}),
        TextPaintError::Source(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn located_paint_cancellation_keeps_its_code_and_native_run() {
        let location = TextPaintLocation {
            paragraph: 3,
            run: 2,
            source_ordinal: 51,
        };
        let e = page_failure(SourcePageError::AtObject {
            location: crate::SourcePageLocation {
                part: "/ppt/slides/slide1.xml".into(),
                object: Some(42),
            },
            error: Box::new(SourcePageError::TextPaint(
                TextPaintError::Source(mo_pptx::PptxError::Cancelled).at_run(location.clone()),
            )),
        });
        let PptxTextPageFailure::Page {
            error,
            paint_location,
            detail,
        } = e
        else {
            panic!("page failure")
        };
        assert!(matches!(error.code, crate::PptxPageFailureCode::Cancelled));
        assert_eq!(paint_location, Some(location));
        assert_eq!(error.location.unwrap().object, Some(42));
        assert!(detail.is_none());
    }
}
