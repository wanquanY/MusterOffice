//! Source-bound solid character paint. Uses the shared color engine and keeps
//! declaration identity; this does not infer glyph ownership or render pixels.
use super::{cascade::*, *};
use crate::{
    PptxError, cancelled,
    source::{color::*, *},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TextPaint {
    None {
        declaration: TextStyleDeclaration,
    },
    Solid {
        declaration: TextStyleDeclaration,
        color: ColorSample,
        placeholder: Option<TextStyleDeclaration>,
        dependencies: Vec<ColorDependency>,
        notices: Vec<ColorNotice>,
    },
}
/// Paint channels retain native declarations independently. A follow-text
/// underline borrows the resolved glyph fill instead of cloning/evaluating it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRunPaint {
    pub fill: TextPaint,
    pub underline: Option<UnderlinePaint>,
    pub strike: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UnderlinePaint {
    FollowText {
        declaration: Option<TextStyleDeclaration>,
    },
    Independent {
        declaration: TextStyleDeclaration,
        fill: Box<TextPaint>,
    },
}
impl TextRunPaint {
    pub fn underline_fill(&self) -> Option<&TextPaint> {
        self.underline.as_ref().map(|p| match p {
            UnderlinePaint::FollowText { .. } => &self.fill,
            UnderlinePaint::Independent { fill, .. } => fill,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextPaintLocation {
    pub paragraph: u32,
    pub run: u32,
    pub source_ordinal: u32,
}
impl TextPaintLocation {
    pub fn at(paragraph: u32, run: &CascadedTextRun) -> Self {
        Self {
            paragraph,
            run: run.run,
            source_ordinal: run.source_ordinal,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TextPaintError {
    #[error("text paint at {location:?}: {error}")]
    AtRun {
        location: TextPaintLocation,
        error: Box<TextPaintError>,
    },
    #[error(transparent)]
    Source(#[from] PptxError),
    #[error("text paint property requires mapping: {0:?}")]
    Property(CharacterProperty),
    #[error("text paint declaration requires mapping: {0:?}")]
    Declaration(Box<TextStyleDeclaration>),
    #[error("unresolved text color at {declaration:?}: {reason:?}")]
    Color {
        declaration: Box<TextStyleDeclaration>,
        reason: ColorUnresolved,
    },
    #[error("native text has no resolved fill or font reference color")]
    Missing,
}
impl TextPaintError {
    pub fn at_run(self, location: TextPaintLocation) -> Self {
        Self::AtRun {
            location,
            error: Box::new(self),
        }
    }
}
fn unsupported(r: &TextStyleDeclaration) -> TextPaintError {
    TextPaintError::Declaration(Box::new(r.clone()))
}
fn font_color<'a>(
    index: &'a SourceIndex,
    r: &TextStyleDeclaration,
) -> Result<(TextStyleDeclaration, &'a SourceColor), TextPaintError> {
    if matches!(r.origin, TextStyleOrigin::TableStyle { .. }) {
        let TableTextDeclaration::Font(
            crate::source::table::styles::SourceTableFontStyle::Reference {
                color: Some(color),
                ..
            },
        ) = table_declaration(index, r)?
        else {
            return Err(unsupported(r));
        };
        return Ok((
            TextStyleDeclaration {
                element: color_element(color),
                origin: r.origin.at(color.source_ordinal),
            },
            color,
        ));
    }
    let n = cascade::declaration(index, r)?;
    if !matches!(n.value, SourceTextValue::FontReference { .. })
        || !n.retained_ordinals.is_empty()
        || n.children.len() != 1
    {
        return Err(unsupported(r));
    }
    let (child, n) = cascade::declaration_child(index, r, n.children[0])?;
    if !n.retained_ordinals.is_empty() {
        return Err(unsupported(&child));
    }
    match &n.value {
        SourceTextValue::Color { color } => Ok((child, color)),
        _ => Err(unsupported(r)),
    }
}
/// A compiler supplies its own freshly prepared cascade. Public wire operations
/// must inspect the source themselves, never deserialize a trusted cascade.
pub fn resolve(
    index: &SourceIndex,
    text: &CascadedText,
    context: &ColorContext,
    limits: ColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Vec<TextRunPaint>>, TextPaintError> {
    cancelled(check)?;
    if index.source_sha256 != text.source_sha256 {
        return Err(PptxError::SourceConflict("text paint source digest".into()).into());
    }
    let surface = index
        .surfaces
        .get(&text.object.part)
        .ok_or_else(|| PptxError::SourceConflict("text paint drawing surface".into()))?;
    let mut session = Session::new(
        index,
        surface,
        ColorProfile::Ecma3762016DraftV1,
        context,
        limits,
        check,
    )?;
    let mut remaining = limits.max_queries;
    let mut result = Vec::new();
    for (paragraph, p) in text.paragraphs.iter().enumerate() {
        let mut paints = Vec::new();
        for run in &p.runs {
            cancelled(check)?;
            charge(&mut remaining)?;
            paints.push(
                resolve_run(index, text, &run.style, &mut session, &mut remaining).map_err(
                    |e| {
                        e.at_run(TextPaintLocation::at(
                            text.paragraph_start + paragraph as u32,
                            run,
                        ))
                    },
                )?,
            );
        }
        result.push(paints);
    }
    cancelled(check)?;
    Ok(result)
}

fn charge(remaining: &mut usize) -> Result<(), TextPaintError> {
    *remaining = remaining
        .checked_sub(1)
        .ok_or(PptxError::Limit("text paint slots"))?;
    Ok(())
}

fn resolve_run(
    index: &SourceIndex,
    text: &CascadedText,
    s: &CascadedCharacterStyle,
    session: &mut Session<'_>,
    remaining: &mut usize,
) -> Result<TextRunPaint, TextPaintError> {
    if !matches!(
        s.attributes.underline,
        Some(NativeTextUnderline::None | NativeTextUnderline::Sng)
    ) {
        return Err(TextPaintError::Property(CharacterProperty::Underline));
    }
    if !matches!(
        s.attributes.strike,
        Some(NativeTextStrike::NoStrike | NativeTextStrike::SngStrike)
    ) {
        return Err(TextPaintError::Property(CharacterProperty::Strike));
    }
    for (slot, r) in &s.declarations {
        match slot {
            CharacterSlot::UnderlineLine
                if s.attributes.underline == Some(NativeTextUnderline::Sng) =>
            {
                let n = cascade::declaration(index, r)?;
                if !n.retained_ordinals.is_empty()
                    || !n.children.is_empty()
                    || !matches!(
                        (slot, n.element),
                        (CharacterSlot::UnderlineLine, NativeTextElement::ULnTx)
                    )
                {
                    return Err(unsupported(r));
                }
            }
            CharacterSlot::Effects => {
                let n = cascade::declaration(index, r)?;
                if !n.retained_ordinals.is_empty()
                    || !matches!(&n.value, SourceTextValue::Effects{effects} if effects.is_explicitly_empty_list())
                {
                    return Err(unsupported(r));
                }
            }
            CharacterSlot::Line
            | CharacterSlot::Highlight
            | CharacterSlot::Click
            | CharacterSlot::MouseOver => return Err(unsupported(r)),
            _ => (),
        }
    }
    let fill = if let Some(r) = s.declarations.get(&CharacterSlot::Fill) {
        resolve_fill(index, text, r, session)?
    } else {
        let r = text
            .font_reference
            .as_ref()
            .ok_or(TextPaintError::Missing)?;
        let (declaration, color) = font_color(index, r)?;
        solid(index, text, declaration, color, session)?
    };
    let underline = if s.attributes.underline == Some(NativeTextUnderline::Sng) {
        Some(match s.declarations.get(&CharacterSlot::UnderlineFill) {
            None => UnderlinePaint::FollowText { declaration: None },
            Some(r) => {
                let n = cascade::declaration(index, r)?;
                if !n.retained_ordinals.is_empty()
                    || !matches!(n.value, SourceTextValue::Container {})
                {
                    return Err(unsupported(r));
                }
                match n.element {
                    NativeTextElement::UFillTx if n.children.is_empty() => {
                        UnderlinePaint::FollowText {
                            declaration: Some(r.clone()),
                        }
                    }
                    NativeTextElement::UFill if n.children.len() == 1 => {
                        charge(remaining)?;
                        let (child, _) = cascade::declaration_child(index, r, n.children[0])?;
                        UnderlinePaint::Independent {
                            declaration: r.clone(),
                            fill: Box::new(resolve_fill(index, text, &child, session)?),
                        }
                    }
                    _ => return Err(unsupported(r)),
                }
            }
        })
    } else {
        None
    };
    Ok(TextRunPaint {
        fill,
        underline,
        strike: s.attributes.strike == Some(NativeTextStrike::SngStrike),
    })
}

fn resolve_fill(
    index: &SourceIndex,
    text: &CascadedText,
    r: &TextStyleDeclaration,
    session: &mut Session<'_>,
) -> Result<TextPaint, TextPaintError> {
    if matches!(r.origin, TextStyleOrigin::TableStyle { .. }) {
        let TableTextDeclaration::Color(color) = table_declaration(index, r)? else {
            return Err(unsupported(r));
        };
        return solid(index, text, r.clone(), color, session);
    }
    let node = cascade::declaration(index, r)?;
    let SourceTextValue::Fill { fill } = &node.value else {
        return Err(unsupported(r));
    };
    if !node.retained_ordinals.is_empty() || !fill.retained_ordinals.is_empty() {
        return Err(unsupported(r));
    }
    let color = match &fill.definition {
        SourceFillDefinition::None {} => {
            return Ok(TextPaint::None {
                declaration: r.clone(),
            });
        }
        SourceFillDefinition::Solid { color: Some(c) } => c,
        _ => return Err(unsupported(r)),
    };
    solid(index, text, r.clone(), color, session)
}

fn solid(
    index: &SourceIndex,
    text: &CascadedText,
    declaration: TextStyleDeclaration,
    color: &SourceColor,
    session: &mut Session<'_>,
) -> Result<TextPaint, TextPaintError> {
    // Look up fontRef only if phClr is actually consulted. An irrelevant
    // unavailable reference must not invalidate an explicit RGB fill.
    let mut placeholder = None;
    let mut lookup = |budget: &mut crate::source::color::Budget<'_>| {
        budget.step()?;
        let Some(r) = &text.font_reference else {
            return Ok(None);
        };
        let (origin, color) = font_color(index, r).map_err(|error| match error {
            TextPaintError::Source(e) => crate::source::color::Failure::Abort(e),
            _ => crate::source::color::Failure::Unresolved(
                ColorUnresolved::RetainedPlaceholderContext {
                    part: text.object.part.clone(),
                    source_ordinal: match r.origin {
                        TextStyleOrigin::TableStyle { source_ordinal, .. }
                        | TextStyleOrigin::Chart { source_ordinal, .. }
                        | TextStyleOrigin::Object { source_ordinal, .. }
                        | TextStyleOrigin::Master { source_ordinal, .. }
                        | TextStyleOrigin::Presentation { source_ordinal, .. }
                        | TextStyleOrigin::Theme { source_ordinal, .. } => source_ordinal,
                        TextStyleOrigin::ProfileDefault {} => 0,
                    },
                },
            ),
        })?;
        placeholder = Some(origin);
        Ok(Some(ExpressionRef {
            value: &color.value,
            transforms: &color.transforms,
        }))
    };
    let evaluation = session.expression_with_placeholder(
        ExpressionRef {
            value: &color.value,
            transforms: &color.transforms,
        },
        Placeholder::Deferred(&mut lookup),
    )?;
    Ok(TextPaint::Solid {
        declaration,
        color: ColorSample::from_computed(evaluation.color),
        placeholder,
        dependencies: evaluation.dependencies,
        notices: evaluation.notices,
    })
}
