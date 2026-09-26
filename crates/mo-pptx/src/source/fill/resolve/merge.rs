//! Type-gated property merging, independent of source graph traversal.
mod gradient;
mod image;
mod rectangle;
use super::{
    Failure,
    budget::{Budget, Lexical},
    types::*,
};
use crate::{
    PptxError,
    source::{drawingml::*, fill::*},
};
use gradient::Gradient;
use image::Image;
use rectangle::InheritedRect;

#[derive(Clone, Copy)]
pub(super) enum Input<'a> {
    None,
    Solid(Option<&'a SourceColor>),
    Gradient(&'a SourceGradientFill),
    Pattern(&'a SourcePatternFill),
    Image(&'a SourceImageFill),
    Group,
}
impl<'a> From<&'a SourceFillDefinition> for Input<'a> {
    fn from(v: &'a SourceFillDefinition) -> Self {
        match v {
            SourceFillDefinition::None {} => Self::None,
            SourceFillDefinition::Solid { color } => Self::Solid(color.as_ref()),
            SourceFillDefinition::Gradient(v) => Self::Gradient(v),
            SourceFillDefinition::Pattern(v) => Self::Pattern(v),
            SourceFillDefinition::Image(v) => Self::Image(v),
            SourceFillDefinition::Group {} => Self::Group,
        }
    }
}
enum Fields {
    None,
    Solid(Option<Box<FillColorExpression>>),
    Gradient(Box<Gradient>),
    Pattern(Box<Pattern>),
    Image(Box<Image>),
    Group,
}
#[derive(Default)]
pub(super) struct Partial {
    origin: Option<FillOrigin>,
    fields: Option<Fields>,
}
impl Partial {
    pub fn group(&self) -> Option<&FillOrigin> {
        matches!(self.fields, Some(Fields::Group))
            .then_some(self.origin.as_ref())
            .flatten()
    }
    pub fn complete(&self) -> bool {
        match &self.fields {
            None => false,
            Some(Fields::None | Fields::Group) => true,
            Some(Fields::Solid(c)) => c.is_some(),
            Some(Fields::Gradient(g)) => g.complete(),
            Some(Fields::Pattern(p)) => {
                p.preset.is_some() && p.foreground.is_some() && p.background.is_some()
            }
            Some(Fields::Image(i)) => i.complete(),
        }
    }
    pub fn merge(
        &mut self,
        input: Input<'_>,
        retained: &[u32],
        origin: &FillOrigin,
        context: Option<&FillOwner>,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        budget.step()?;
        if self.complete() {
            return Ok(());
        }
        let matches = matches!(
            (&self.fields, input),
            (None, _)
                | (Some(Fields::Solid(_)), Input::Solid(_))
                | (Some(Fields::Gradient(_)), Input::Gradient(_))
                | (Some(Fields::Pattern(_)), Input::Pattern(_))
                | (Some(Fields::Image(_)), Input::Image(_))
        );
        if !matches {
            return Ok(());
        }
        if let Some(ordinal) = retained.first() {
            return Err(FillUnresolved::RetainedContent {
                origin: budget.at(origin, *ordinal)?,
            }
            .into());
        }
        if self.fields.is_none() {
            self.origin = Some(budget.origin(origin)?);
            self.fields = Some(match input {
                Input::None => Fields::None,
                Input::Solid(_) => Fields::Solid(None),
                Input::Gradient(_) => Fields::Gradient(Box::default()),
                Input::Pattern(_) => Fields::Pattern(Box::default()),
                Input::Image(_) => Fields::Image(Box::default()),
                Input::Group => Fields::Group,
            });
        }
        match (self.fields.as_mut().expect("selected fill"), input) {
            (Fields::Solid(slot), Input::Solid(Some(c))) if slot.is_none() => {
                *slot = Some(Box::new(color(c, origin, context, budget)?))
            }
            (Fields::Gradient(g), Input::Gradient(v)) => g.merge(v, origin, context, budget)?,
            (Fields::Pattern(p), Input::Pattern(v)) => p.merge(v, origin, context, budget)?,
            (Fields::Image(i), Input::Image(v)) => i.merge(v, origin, budget)?,
            _ => (),
        }
        Ok(())
    }
    pub fn finish(self, budget: &mut Budget<'_>) -> Result<EffectiveFill, Failure> {
        budget.step()?;
        budget.values(32)?;
        budget.bytes(64)?;
        let declared_by = self.origin.unwrap_or(FillOrigin::ProfileDefault {});
        Ok(match self.fields.unwrap_or(Fields::None) {
            Fields::None => EffectiveFill::None { declared_by },
            Fields::Solid(c) => EffectiveFill::Solid {
                declared_by,
                color: c.unwrap_or_else(|| {
                    Box::new(default_color(SourceColorValue::Scheme {
                        slot: SchemeColor::Bg1,
                    }))
                }),
            },
            Fields::Gradient(g) => EffectiveFill::Gradient {
                declared_by,
                gradient: Box::new(g.finish(budget)?),
            },
            Fields::Pattern(p) => EffectiveFill::Pattern {
                declared_by,
                pattern: Box::new(p.finish()),
            },
            Fields::Image(i) => EffectiveFill::Image {
                image: Box::new(i.finish(&declared_by, budget)?),
                declared_by,
            },
            Fields::Group => {
                return Err(FillUnresolved::GroupWithoutParent {
                    origin: declared_by,
                }
                .into());
            }
        })
    }
}
pub(super) fn color(
    c: &SourceColor,
    origin: &FillOrigin,
    context: Option<&FillOwner>,
    budget: &mut Budget<'_>,
) -> Result<FillColorExpression, Failure> {
    let color = budget.color(c, origin)?;
    Ok(FillColorExpression {
        color,
        context_owner: context.map(|owner| budget.owner(owner)).transpose()?,
    })
}
fn default_color(value: SourceColorValue) -> FillColorExpression {
    FillColorExpression {
        color: FillColorTerm {
            value,
            transforms: vec![],
            declared_by: FillOrigin::ProfileDefault {},
        },
        context_owner: None,
    }
}
fn default<T>(value: T) -> FillValue<T> {
    FillValue {
        value,
        declared_by: FillOrigin::ProfileDefault {},
    }
}
fn percentage(value: &str) -> NativePercentage {
    value
        .to_owned()
        .try_into()
        .expect("fixed native percentage default")
}
fn set<T: Clone + Lexical>(
    slot: &mut Option<FillValue<T>>,
    value: &Option<T>,
    origin: &FillOrigin,
    budget: &mut Budget<'_>,
) -> Result<(), PptxError> {
    if slot.is_none() {
        *slot = value.as_ref().map(|v| budget.bind(v, origin)).transpose()?;
    }
    Ok(())
}
/// tileRect and stretch/fillRect receive schema defaults at the selected node.
/// srcRect and fillToRect use per-edge inheritance in rectangle::InheritedRect.
fn rect(
    value: &SourceFillRect,
    origin: &FillOrigin,
    budget: &mut Budget<'_>,
) -> Result<EffectiveFillRect, PptxError> {
    let origin = budget.at(origin, value.source_ordinal)?;
    let schema = FillOrigin::SchemaDefault {
        part: origin.part().unwrap_or_default().into(),
        source_ordinal: value.source_ordinal,
    };
    let mut edge = |v: &Option<NativePercentage>| match v {
        Some(v) => budget.bind(v, &origin),
        None => budget.bind(&percentage("0"), &schema),
    };
    Ok(EffectiveFillRect {
        left: edge(&value.left)?,
        top: edge(&value.top)?,
        right: edge(&value.right)?,
        bottom: edge(&value.bottom)?,
        declared_by: origin,
    })
}
fn default_rect(value: &str) -> EffectiveFillRect {
    EffectiveFillRect {
        declared_by: FillOrigin::ProfileDefault {},
        left: default(percentage(value)),
        top: default(percentage(value)),
        right: default(percentage(value)),
        bottom: default(percentage(value)),
    }
}
#[derive(Default)]
struct Pattern {
    preset: Option<FillValue<NativePattern>>,
    foreground: Option<FillColorExpression>,
    background: Option<FillColorExpression>,
}
impl Pattern {
    fn merge(
        &mut self,
        v: &SourcePatternFill,
        origin: &FillOrigin,
        context: Option<&FillOwner>,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        set(&mut self.preset, &v.preset, origin, budget)?;
        for (slot, input) in [
            (&mut self.foreground, &v.foreground),
            (&mut self.background, &v.background),
        ] {
            if slot.is_none()
                && let Some(input) = input
            {
                *slot = Some(color(&input.color, origin, context, budget)?);
            }
        }
        Ok(())
    }
    fn finish(self) -> EffectivePatternFill {
        EffectivePatternFill {
            preset: self.preset.unwrap_or_else(|| default(NativePattern::Pct5)),
            foreground: self
                .foreground
                .unwrap_or_else(|| default_color(SourceColorValue::Srgb { rgb: [0, 0, 0] })),
            background: self.background.unwrap_or_else(|| {
                default_color(SourceColorValue::Srgb {
                    rgb: [255, 255, 255],
                })
            }),
        }
    }
}
