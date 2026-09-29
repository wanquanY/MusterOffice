//! Typed author paint declarations. No XML serialization or parsing is involved.
use super::{Ordinals, native, *};
use crate::source::{drawingml::*, fill::*, line::*};
use mo_presentation_model as model;

pub(super) fn color(value: &model::Color, ord: &mut Ordinals) -> Result<SourceColor, PptxError> {
    let source_ordinal = ord.next()?;
    let mut transforms = Vec::new();
    let value = match value {
        model::Color::Srgb { rgba } => {
            if rgba.alpha != 255 {
                let alpha = (u32::from(rgba.alpha) * 100000 + 127) / 255;
                transforms.push(SourceColorTransform::Alpha(native(alpha.to_string())?));
            }
            SourceColorValue::Srgb {
                rgb: [rgba.red, rgba.green, rgba.blue],
            }
        }
        model::Color::Theme { slot } => SourceColorValue::Scheme {
            slot: native(slot_name(*slot).into())?,
        },
    };
    Ok(SourceColor {
        source_ordinal,
        value,
        transforms,
    })
}
pub(super) fn slot_name(slot: model::ThemeColor) -> &'static str {
    COLOR_SLOTS
        .iter()
        .find(|(s, _)| *s == slot)
        .expect("exhaustive theme colors")
        .1
}
pub(super) fn fill(value: &model::Fill, ord: &mut Ordinals) -> Result<SourceFill, PptxError> {
    let source_ordinal = ord.next()?;
    let definition = match value {
        model::Fill::None => SourceFillDefinition::None {},
        model::Fill::Solid { color: value } => SourceFillDefinition::Solid {
            color: Some(color(value, ord)?),
        },
    };
    Ok(SourceFill {
        source_ordinal,
        definition,
        retained_ordinals: vec![],
    })
}
pub(super) fn object_line(
    object: &model::Object,
    ord: &mut Ordinals,
) -> Result<Option<SourceLine>, PptxError> {
    let model::Inherited::Value(stroke) = &object.appearance.stroke else {
        return Ok(None);
    };
    if matches!(
        object.content,
        model::ObjectContent::Group { .. } | model::ObjectContent::Table { .. }
    ) {
        // CT_GroupShapeProperties has no line or geometry. An explicit absence
        // describes the group's own outline, not a style mutation of its children.
        // Normalize only the native projection; the author declaration and its
        // identity remain intact. A visible stroke still needs a defined mapping.
        return match stroke {
            model::Stroke::None {} => Ok(None),
            model::Stroke::Solid { .. } => Err(PptxError::Unsupported(
                if matches!(object.content, model::ObjectContent::Group { .. }) {
                    "group stroke semantics"
                } else {
                    "table frame stroke semantics"
                }
                .into(),
            )),
        };
    }
    line(stroke, ord).map(Some)
}
pub(super) fn line(value: &model::Stroke, ord: &mut Ordinals) -> Result<SourceLine, PptxError> {
    let source_ordinal = ord.next()?;
    let mut out = SourceLine {
        source_ordinal,
        width: None,
        cap: None,
        compound: None,
        alignment: None,
        fill: None,
        dash: None,
        join: None,
        head: None,
        tail: None,
        retained_ordinals: vec![],
    };
    match value {
        model::Stroke::None {} => {
            out.fill = Some(SourceLineFill::None {
                source_ordinal: ord.next()?,
            })
        }
        model::Stroke::Solid {
            color: value,
            width,
            cap,
            join,
        } => {
            if !(0..=20116800).contains(&width.get()) {
                return Err(value_error("stroke.width", "outside native range"));
            }
            out.width = Some(*width);
            out.cap = cap.map(|v| match v {
                model::LineCap::Flat => NativeLineCap::Flat,
                model::LineCap::Round => NativeLineCap::Round,
                model::LineCap::Square => NativeLineCap::Square,
            });
            out.fill = Some(SourceLineFill::Solid {
                source_ordinal: ord.next()?,
                color: Some(color(value, ord)?),
            });
            out.join = match join {
                None => None,
                Some(model::LineJoin::Round {}) => Some(SourceLineJoin::Round {
                    source_ordinal: ord.next()?,
                }),
                Some(model::LineJoin::Bevel {}) => Some(SourceLineJoin::Bevel {
                    source_ordinal: ord.next()?,
                }),
                Some(model::LineJoin::Miter { limit }) => Some(SourceLineJoin::Miter {
                    source_ordinal: ord.next()?,
                    limit: limit.map(|v| native(v.to_string())).transpose()?,
                }),
            };
        }
    }
    Ok(out)
}
pub(super) fn image(
    reference: String,
    crop: model::Crop,
    ord: &mut Ordinals,
) -> Result<SourceFill, PptxError> {
    let source_ordinal = ord.next()?;
    let blip = SourceFillBlip {
        source_ordinal: ord.next()?,
        embed: Some(reference),
        link: None,
        compression: None,
        effect_nodes: vec![],
        retained_ordinals: vec![],
    };
    let mut percentages: Vec<NativePercentage> = Vec::new();
    for value in [crop.left, crop.top, crop.right, crop.bottom] {
        if value % 10 != 0 {
            return Err(value_error(
                "picture crop",
                "millionths are not exactly representable in native units",
            ));
        }
        percentages.push(native((value / 10).to_string())?);
    }
    let rect = SourceFillRect {
        source_ordinal: ord.next()?,
        left: Some(percentages[0].clone()),
        top: Some(percentages[1].clone()),
        right: Some(percentages[2].clone()),
        bottom: Some(percentages[3].clone()),
    };
    let mode = SourceImageFillMode::Stretch {
        source_ordinal: ord.next()?,
        fill_rect: Some(SourceFillRect {
            source_ordinal: ord.next()?,
            left: None,
            top: None,
            right: None,
            bottom: None,
        }),
    };
    Ok(SourceFill {
        source_ordinal,
        definition: SourceFillDefinition::Image(SourceImageFill {
            blip: Some(blip),
            source_rect: Some(rect),
            mode: Some(mode),
            dpi: None,
            rotate_with_shape: None,
        }),
        retained_ordinals: vec![],
    })
}
