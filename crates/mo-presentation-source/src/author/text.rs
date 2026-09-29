//! Author text lowered to the same typed declaration graph used by imported
//! presentations. The common cascade, font selection and layout own defaults.
use super::*;
use crate::source::{SourceRun, SourceRunKind, drawingml::SourceTextFont, text::*};
use NativeTextElement as N;
use mo_presentation_model::*;

pub(super) struct TextBuilder<'a, 'c> {
    pub catalog: SourceTextCatalog,
    pub ord: &'a mut Ordinals<'c>,
}
pub(super) fn font(family: &str) -> Result<SourceTextFont, PptxError> {
    if family.trim().is_empty() {
        return Err(value_error(
            "fontFamily",
            "explicit font family is required",
        ));
    }
    Ok(SourceTextFont {
        typeface: family.into(),
        panose: None,
        pitch_family: None,
        charset: None,
    })
}
pub(super) fn centipoints(
    value: mo_common::Emu,
    field: &str,
    min: i64,
    max: i64,
) -> Result<i32, PptxError> {
    if value.get() % 127 != 0 || !(min..=max).contains(&(value.get() / 127)) {
        return Err(value_error(
            field,
            "value is not exactly representable in native hundredths of a point",
        ));
    }
    Ok((value.get() / 127) as i32)
}
fn explicit<T>(value: &Inherited<T>) -> Option<&T> {
    match value {
        Inherited::Inherit => None,
        Inherited::Value(value) => Some(value),
    }
}
impl<'a, 'c> TextBuilder<'a, 'c> {
    pub fn new(ord: &'a mut Ordinals<'c>) -> Self {
        Self {
            catalog: Default::default(),
            ord,
        }
    }
    fn insert(&mut self, id: u32, element: N, parent: Option<u32>, value: SourceTextValue) -> u32 {
        self.catalog.nodes.insert(
            id,
            SourceTextNode {
                element,
                parent,
                children: vec![],
                value,
                retained_ordinals: vec![],
            },
        );
        if let Some(parent) = parent {
            self.catalog
                .nodes
                .get_mut(&parent)
                .expect("constructed text parent")
                .children
                .push(id);
        }
        id
    }
    fn node(
        &mut self,
        element: N,
        parent: Option<u32>,
        value: SourceTextValue,
    ) -> Result<u32, PptxError> {
        let id = self.ord.next()?;
        Ok(self.insert(id, element, parent, value))
    }
    fn container(&mut self, element: N, parent: Option<u32>) -> Result<u32, PptxError> {
        self.node(element, parent, SourceTextValue::Container {})
    }
    fn root(&mut self, element: N, owner: Option<u32>) -> Result<u32, PptxError> {
        let id = self.container(element, None)?;
        self.catalog.roots.push(SourceTextRoot {
            cell: None,
            source_ordinal: id,
            owner,
        });
        Ok(id)
    }
    fn paragraph_style(
        &mut self,
        element: N,
        parent: u32,
        attributes: SourceTextParagraphAttributes,
    ) -> Result<u32, PptxError> {
        self.node(
            element,
            Some(parent),
            SourceTextValue::Paragraph {
                attributes: Box::new(attributes),
            },
        )
    }
    fn fonts(&mut self, parent: u32, family: &str) -> Result<(), PptxError> {
        for element in [N::Latin, N::Ea, N::Cs] {
            self.node(
                element,
                Some(parent),
                SourceTextValue::Font {
                    font: Box::new(font(family)?),
                },
            )?;
        }
        Ok(())
    }
    fn fill(&mut self, parent: u32, color: &Color) -> Result<(), PptxError> {
        let fill = super::paint::fill(
            &Fill::Solid {
                color: color.clone(),
            },
            self.ord,
        )?;
        self.insert(
            fill.source_ordinal,
            N::SolidFill,
            Some(parent),
            SourceTextValue::Fill {
                fill: Box::new(fill),
            },
        );
        Ok(())
    }
    fn character(
        &mut self,
        parent: u32,
        element: N,
        style: &CharacterStyle,
        document: &Document,
    ) -> Result<u32, PptxError> {
        let attributes = SourceTextCharacterAttributes {
            size: explicit(&style.size)
                .map(|s| centipoints(*s, "font size", 100, 400000))
                .transpose()?,
            bold: explicit(&style.bold).copied(),
            italic: explicit(&style.italic).copied(),
            underline: explicit(&style.underline).map(|v| {
                if *v {
                    NativeTextUnderline::Sng
                } else {
                    NativeTextUnderline::None
                }
            }),
            language: explicit(&style.language).cloned(),
            ..Default::default()
        };
        let id = self.node(
            element,
            Some(parent),
            SourceTextValue::Character {
                attributes: Box::new(attributes),
            },
        )?;
        if let Some(color) = explicit(&style.color) {
            self.fill(id, color)?;
        }
        if let Some(font) = explicit(&style.font) {
            self.fonts(id, &document.fonts[font].family)?;
        }
        Ok(id)
    }
    pub fn defaults(&mut self, defaults: &ExportDefaults) -> Result<(), PptxError> {
        let root = self.root(N::DefaultTextStyle, None)?;
        let level = self.paragraph_style(N::Lvl1pPr, root, Default::default())?;
        let id = self.node(
            N::DefRPr,
            Some(level),
            SourceTextValue::Character {
                attributes: Box::new(SourceTextCharacterAttributes {
                    size: Some(centipoints(
                        defaults.text_size,
                        "defaults.textSize",
                        100,
                        400000,
                    )?),
                    ..Default::default()
                }),
            },
        )?;
        self.fill(
            id,
            &Color::Srgb {
                rgba: defaults.text_color,
            },
        )?;
        self.fonts(id, &defaults.font_family)
    }
    pub fn master(
        &mut self,
        style: Option<&CharacterStyle>,
        document: &Document,
    ) -> Result<(), PptxError> {
        let root = self.root(N::TxStyles, None)?;
        for kind in [N::TitleStyle, N::BodyStyle, N::OtherStyle] {
            let kind = self.container(kind, Some(root))?;
            if let Some(style) = style {
                let level = self.paragraph_style(N::Lvl1pPr, kind, Default::default())?;
                self.character(level, N::DefRPr, style, document)?;
            }
        }
        Ok(())
    }
    pub fn cell_body(
        &mut self,
        owner: u32,
        cell: crate::source::table::SourceCellAddress,
        body: Option<&TextBody>,
        document: &Document,
    ) -> Result<(u32, Vec<Vec<SourceRun>>), PptxError> {
        let result = if let Some(body) = body {
            self.body(owner, body, document)?
        } else {
            let root = self.root(N::TxBody, Some(owner))?;
            self.node(
                N::BodyPr,
                Some(root),
                SourceTextValue::Body {
                    attributes: Box::default(),
                },
            )?;
            self.container(N::LstStyle, Some(root))?;
            self.container(N::P, Some(root))?;
            (root, vec![vec![]])
        };
        self.catalog
            .roots
            .iter_mut()
            .find(|r| r.source_ordinal == result.0)
            .expect("created cell root")
            .cell = Some(cell);
        let children = self.catalog.nodes[&result.0].children.clone();
        for child in children {
            if let SourceTextValue::Body { attributes } = &mut self
                .catalog
                .nodes
                .get_mut(&child)
                .expect("body child")
                .value
            {
                attributes.left_inset = None;
                attributes.top_inset = None;
                attributes.right_inset = None;
                attributes.bottom_inset = None;
            }
        }
        Ok(result)
    }
    pub fn body(
        &mut self,
        owner: u32,
        body: &TextBody,
        document: &Document,
    ) -> Result<(u32, Vec<Vec<SourceRun>>), PptxError> {
        let root = self.root(N::TxBody, Some(owner))?;
        let mut vertical = None;
        for p in &body.paragraphs {
            let next = match p.style.direction {
                Inherited::Value(TextDirection::VerticalRightToLeft) => NativeTextVertical::EaVert,
                Inherited::Value(TextDirection::VerticalLeftToRight) => {
                    NativeTextVertical::MongolianVert
                }
                _ => NativeTextVertical::Horz,
            };
            if vertical.is_some_and(|v| v != next) {
                return Err(PptxError::Unsupported(
                    "mixed paragraph writing modes in one native text body".into(),
                ));
            }
            vertical = Some(next);
        }
        let inset = |value: mo_common::Emu| {
            if value.get() > i32::MAX as i64 {
                return Err(value_error("text inset", "outside native range"));
            }
            native(value.get().to_string())
        };
        let clipped = body.overflow == OverflowPolicy::Clip;
        let attributes = SourceTextBodyAttributes {
            left_inset: Some(inset(body.insets.left)?),
            top_inset: Some(inset(body.insets.top)?),
            right_inset: Some(inset(body.insets.right)?),
            bottom_inset: Some(inset(body.insets.bottom)?),
            wrap: Some(if body.wrap {
                NativeTextWrap::Square
            } else {
                NativeTextWrap::None
            }),
            vertical_overflow: Some(if clipped {
                NativeTextVerticalOverflow::Clip
            } else {
                NativeTextVerticalOverflow::Overflow
            }),
            horizontal_overflow: Some(if clipped {
                NativeTextHorizontalOverflow::Clip
            } else {
                NativeTextHorizontalOverflow::Overflow
            }),
            vertical: Some(vertical.unwrap_or(NativeTextVertical::Horz)),
            ..Default::default()
        };
        let properties = self.node(
            N::BodyPr,
            Some(root),
            SourceTextValue::Body {
                attributes: Box::new(attributes),
            },
        )?;
        self.container(
            if body.overflow == OverflowPolicy::GrowShape {
                N::SpAutoFit
            } else {
                N::NoAutofit
            },
            Some(properties),
        )?;
        let list = self.container(N::LstStyle, Some(root))?;
        let level = self.paragraph_style(N::Lvl1pPr, list, Default::default())?;
        self.character(level, N::DefRPr, &body.style, document)?;
        let mut paragraphs = Vec::new();
        for p in &body.paragraphs {
            let parent = self.container(N::P, Some(root))?;
            let rtl = matches!(
                p.style.direction,
                Inherited::Value(TextDirection::RightToLeft)
            );
            let alignment = explicit(&p.style.alignment).map(|a| match a {
                Alignment::Start => {
                    if rtl {
                        NativeTextAlign::R
                    } else {
                        NativeTextAlign::L
                    }
                }
                Alignment::End => {
                    if rtl {
                        NativeTextAlign::L
                    } else {
                        NativeTextAlign::R
                    }
                }
                Alignment::Center => NativeTextAlign::Ctr,
                Alignment::Justify => NativeTextAlign::Just,
            });
            let direction = match p.style.direction {
                Inherited::Value(TextDirection::LeftToRight | TextDirection::RightToLeft) => {
                    Some(rtl)
                }
                _ => None,
            };
            let properties = self.paragraph_style(
                N::PPr,
                parent,
                SourceTextParagraphAttributes {
                    alignment,
                    right_to_left: direction,
                    ..Default::default()
                },
            )?;
            for (element, value) in [
                (N::SpcBef, &p.style.space_before),
                (N::SpcAft, &p.style.space_after),
            ] {
                if let Some(value) = explicit(value) {
                    let span = self.container(element, Some(properties))?;
                    self.node(
                        N::SpcPts,
                        Some(span),
                        SourceTextValue::Points {
                            value: centipoints(*value, "paragraph spacing", 0, 158400)?,
                        },
                    )?;
                }
            }
            self.character(properties, N::DefRPr, &p.default_run_style, document)?;
            let mut runs = Vec::new();
            for run in &p.runs {
                let (element, kind, text) = match &run.content {
                    InlineContent::Break => (N::Br, SourceRunKind::Break, String::new()),
                    InlineContent::Tab => (N::R, SourceRunKind::Text, "\t".into()),
                    InlineContent::Text { text } => (N::R, SourceRunKind::Text, text.clone()),
                };
                let run_node = self.container(element, Some(parent))?;
                self.character(run_node, N::RPr, &run.style, document)?;
                if kind == SourceRunKind::Text {
                    self.container(N::T, Some(run_node))?;
                }
                runs.push(SourceRun {
                    kind,
                    text,
                    editable: kind == SourceRunKind::Text,
                    edit_constraint: None,
                });
            }
            paragraphs.push(runs);
        }
        Ok((root, paragraphs))
    }
}
