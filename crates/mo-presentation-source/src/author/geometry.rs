use super::*;
use crate::source::geometry::*;
use mo_presentation_model::{Geometry, PathCommand, Point, Size};

fn point(value: Point, ord: &mut Ordinals) -> Result<SourceGeometryPoint, PptxError> {
    Ok(SourceGeometryPoint {
        source_ordinal: ord.next()?,
        x: coordinate(value.x)?.to_string(),
        y: coordinate(value.y)?.to_string(),
    })
}
pub(super) fn coordinate(value: mo_common::Emu) -> Result<i64, PptxError> {
    let n = value.get();
    if !(-27273042329600..=27273042316900).contains(&n) {
        return Err(value_error(
            "coordinate",
            "outside DrawingML coordinate range",
        ));
    }
    Ok(n)
}
pub(super) fn geometry(
    value: &Geometry,
    size: Size,
    ord: &mut Ordinals,
) -> Result<SourceGeometry, PptxError> {
    let source_ordinal = ord.next()?;
    let adjustments = SourceGeometryList {
        source_ordinal: ord.next()?,
        entries: vec![],
    };
    let definition = match value {
        Geometry::Rectangle | Geometry::Ellipse | Geometry::RoundRectangle { .. } => {
            let mut adjustments = adjustments;
            let preset = match value {
                Geometry::Rectangle => "rect",
                Geometry::Ellipse => "ellipse",
                _ => "roundRect",
            };
            if let Geometry::RoundRectangle { radius } = value {
                let short = size.width.get().min(size.height.get());
                let numerator = i128::from(radius.get()) * 100000;
                if short <= 0 || radius.get() > short / 2 || numerator % i128::from(short) != 0 {
                    return Err(value_error(
                        "roundRectangle.radius",
                        "radius is not exactly representable by a native adjustment",
                    ));
                }
                adjustments.entries.push(SourceGuide {
                    source_ordinal: ord.next()?,
                    name: "adj".into(),
                    formula: format!("val {}", numerator / i128::from(short)),
                });
            }
            SourceGeometryDefinition::Preset {
                preset: native(preset.into())?,
                adjustments: Some(adjustments),
            }
        }
        Geometry::Path { commands, viewport } => {
            let guides = Some(SourceGeometryList {
                source_ordinal: ord.next()?,
                entries: vec![],
            });
            let handles = Some(SourceGeometryList {
                source_ordinal: ord.next()?,
                entries: vec![],
            });
            let connections = Some(SourceGeometryList {
                source_ordinal: ord.next()?,
                entries: vec![],
            });
            let text_rect = Some(SourceGeometryRect {
                source_ordinal: ord.next()?,
                left: "0".into(),
                top: "0".into(),
                right: "r".into(),
                bottom: "b".into(),
            });
            let paths_ordinal = ord.next()?;
            let path_ordinal = ord.next()?;
            coordinate(viewport.width)?;
            coordinate(viewport.height)?;
            let mut output = Vec::new();
            for command in commands {
                let source_ordinal = ord.next()?;
                output.push(match command {
                    PathCommand::Move { to } => SourceGeometryCommand::Move {
                        source_ordinal,
                        to: point(*to, ord)?,
                    },
                    PathCommand::Line { to } => SourceGeometryCommand::Line {
                        source_ordinal,
                        to: point(*to, ord)?,
                    },
                    PathCommand::Quadratic { control, to } => SourceGeometryCommand::Quadratic {
                        source_ordinal,
                        control: point(*control, ord)?,
                        to: point(*to, ord)?,
                    },
                    PathCommand::Cubic {
                        control1,
                        control2,
                        to,
                    } => SourceGeometryCommand::Cubic {
                        source_ordinal,
                        control1: point(*control1, ord)?,
                        control2: point(*control2, ord)?,
                        to: point(*to, ord)?,
                    },
                    PathCommand::Close => SourceGeometryCommand::Close { source_ordinal },
                });
            }
            SourceGeometryDefinition::Custom(Box::new(SourceCustomGeometry {
                adjustments: Some(adjustments),
                guides,
                handles,
                connections,
                text_rect,
                paths: SourceGeometryList {
                    source_ordinal: paths_ordinal,
                    entries: vec![SourceGeometryPath {
                        source_ordinal: path_ordinal,
                        width: Some(viewport.width),
                        height: Some(viewport.height),
                        fill: None,
                        stroke: None,
                        extrusion_ok: None,
                        commands: output,
                    }],
                },
            }))
        }
    };
    Ok(SourceGeometry {
        source_ordinal,
        definition,
        retained_ordinals: vec![],
    })
}
