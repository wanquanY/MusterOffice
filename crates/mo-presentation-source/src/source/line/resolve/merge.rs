//! Legacy object-line query joins the shared stroke merger to its fill result.
use super::{super::*, *};
use crate::source::drawingml::*;
pub(super) fn color(c: &SourceColor, source: &LineOrigin) -> LineColorTerm {
    LineColorTerm {
        value: c.value.clone(),
        transforms: c.transforms.clone(),
        declared_by: source.at(c.source_ordinal),
    }
}
enum Fill {
    None(LineOrigin),
    Solid(LineOrigin, Option<Box<LineColorExpression>>),
}
#[derive(Default)]
pub(super) struct Partial {
    geometry: GeometryPartial,
    fill: Option<Fill>,
}
impl Partial {
    pub fn complete(&self) -> bool {
        self.geometry.complete()
            && matches!(self.fill, Some(Fill::None(_) | Fill::Solid(_, Some(_))))
    }
    pub fn merge(
        &mut self,
        input: &SourceLine,
        origin: &LineOrigin,
        placeholder: Option<&LineColorTerm>,
    ) -> Result<(), LineUnresolved> {
        if !input.retained_ordinals.is_empty() && !self.complete() {
            return Err(LineUnresolved::RetainedContent {
                origin: origin.at(input.retained_ordinals[0]),
            });
        }
        self.geometry.merge(input, origin)?;
        if let Some(fill) = &input.fill {
            match fill {
                SourceLineFill::None { source_ordinal } => {
                    if self.fill.is_none() {
                        self.fill = Some(Fill::None(origin.at(*source_ordinal)));
                    }
                }
                SourceLineFill::Solid {
                    source_ordinal,
                    color: native,
                } => {
                    if self.fill.is_none() {
                        self.fill = Some(Fill::Solid(origin.at(*source_ordinal), None));
                    }
                    if let Some(Fill::Solid(_, slot)) = &mut self.fill
                        && slot.is_none()
                    {
                        *slot = native.as_ref().map(|c| {
                            Box::new(LineColorExpression {
                                color: color(c, origin),
                                placeholder: placeholder.cloned(),
                            })
                        });
                    }
                }
                SourceLineFill::Gradient { source_ordinal, .. }
                | SourceLineFill::Pattern { source_ordinal, .. } => {
                    if self.fill.is_none() {
                        return Err(LineUnresolved::UnsupportedFill {
                            origin: origin.at(*source_ordinal),
                            native_kind: if matches!(fill, SourceLineFill::Gradient { .. }) {
                                NativeRetainedLineFill::Gradient
                            } else {
                                NativeRetainedLineFill::Pattern
                            },
                        });
                    }
                }
            }
        }
        Ok(())
    }
    pub fn finish(self, placeholder: Option<LineColorTerm>) -> EffectiveLine {
        let default = LineOrigin::ProfileDefault {};
        let fill = match self.fill.unwrap_or(Fill::None(default.clone())) {
            Fill::None(declared_by) => EffectiveLineFill::None { declared_by },
            Fill::Solid(declared_by, color) => {
                let mut color = color.unwrap_or_else(|| {
                    Box::new(LineColorExpression {
                        color: LineColorTerm {
                            value: SourceColorValue::Scheme {
                                slot: SchemeColor::Bg1,
                            },
                            transforms: vec![],
                            declared_by: default.clone(),
                        },
                        placeholder: None,
                    })
                });
                if color.placeholder.is_none() {
                    color.placeholder = placeholder;
                }
                EffectiveLineFill::Solid { declared_by, color }
            }
        };
        self.geometry.finish().with_fill(fill)
    }
}
