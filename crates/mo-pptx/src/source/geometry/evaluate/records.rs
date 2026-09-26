use super::{Failure, session::Session, types::*};
use crate::source::geometry::*;

impl Session<'_, '_, '_> {
    pub fn rect(&mut self, r: &SourceGeometryRect) -> Result<EvaluatedRect, Failure> {
        Ok(EvaluatedRect {
            origin: self.origin(r.source_ordinal),
            left: self.coordinate(&r.left, r.source_ordinal)?,
            top: self.coordinate(&r.top, r.source_ordinal)?,
            right: self.coordinate(&r.right, r.source_ordinal)?,
            bottom: self.coordinate(&r.bottom, r.source_ordinal)?,
        })
    }
    pub fn handle(&mut self, h: &SourceAdjustHandle) -> Result<EvaluatedHandle, Failure> {
        self.budget.step()?;
        Ok(match h {
            SourceAdjustHandle::Xy {
                source_ordinal,
                position,
                guide_x,
                min_x,
                max_x,
                guide_y,
                min_y,
                max_y,
            } => EvaluatedHandle::Xy {
                origin: self.origin(*source_ordinal),
                position: self.point(position)?,
                guide_x: self.handle_reference(guide_x.as_deref(), *source_ordinal)?,
                min_x: self.optional(min_x.as_deref(), *source_ordinal, false)?,
                max_x: self.optional(max_x.as_deref(), *source_ordinal, false)?,
                guide_y: self.handle_reference(guide_y.as_deref(), *source_ordinal)?,
                min_y: self.optional(min_y.as_deref(), *source_ordinal, false)?,
                max_y: self.optional(max_y.as_deref(), *source_ordinal, false)?,
            },
            SourceAdjustHandle::Polar {
                source_ordinal,
                position,
                guide_radius,
                min_radius,
                max_radius,
                guide_angle,
                min_angle,
                max_angle,
            } => EvaluatedHandle::Polar {
                origin: self.origin(*source_ordinal),
                position: self.point(position)?,
                guide_radius: self.handle_reference(guide_radius.as_deref(), *source_ordinal)?,
                min_radius: self.optional(min_radius.as_deref(), *source_ordinal, false)?,
                max_radius: self.optional(max_radius.as_deref(), *source_ordinal, false)?,
                guide_angle: self.handle_reference(guide_angle.as_deref(), *source_ordinal)?,
                min_angle: self.optional(min_angle.as_deref(), *source_ordinal, true)?,
                max_angle: self.optional(max_angle.as_deref(), *source_ordinal, true)?,
            },
        })
    }
    pub fn connection(&mut self, c: &SourceConnectionSite) -> Result<EvaluatedConnection, Failure> {
        self.budget.step()?;
        Ok(EvaluatedConnection {
            origin: self.origin(c.source_ordinal),
            angle: self.angle(&c.angle, c.source_ordinal)?,
            position: self.point(&c.position)?,
        })
    }
    pub fn path(&mut self, p: &SourceGeometryPath) -> Result<EvaluatedPath, Failure> {
        self.budget.value()?;
        let mut commands = Vec::new();
        for c in &p.commands {
            commands.push(self.command(c)?);
        }
        Ok(EvaluatedPath {
            origin: self.origin(p.source_ordinal),
            width: p.width,
            height: p.height,
            fill: p.fill,
            stroke: p.stroke,
            extrusion_ok: p.extrusion_ok,
            commands,
        })
    }
    fn command(&mut self, c: &SourceGeometryCommand) -> Result<EvaluatedCommand, Failure> {
        self.budget.value()?;
        Ok(match c {
            SourceGeometryCommand::Move { source_ordinal, to } => EvaluatedCommand::Move {
                origin: self.origin(*source_ordinal),
                to: self.point(to)?,
            },
            SourceGeometryCommand::Line { source_ordinal, to } => EvaluatedCommand::Line {
                origin: self.origin(*source_ordinal),
                to: self.point(to)?,
            },
            SourceGeometryCommand::Quadratic {
                source_ordinal,
                control,
                to,
            } => EvaluatedCommand::Quadratic {
                origin: self.origin(*source_ordinal),
                control: self.point(control)?,
                to: self.point(to)?,
            },
            SourceGeometryCommand::Cubic {
                source_ordinal,
                control1,
                control2,
                to,
            } => EvaluatedCommand::Cubic {
                origin: self.origin(*source_ordinal),
                control1: self.point(control1)?,
                control2: self.point(control2)?,
                to: self.point(to)?,
            },
            SourceGeometryCommand::Close { source_ordinal } => EvaluatedCommand::Close {
                origin: self.origin(*source_ordinal),
            },
            SourceGeometryCommand::Arc {
                source_ordinal,
                width_radius,
                height_radius,
                start_angle,
                sweep_angle,
            } => EvaluatedCommand::Arc {
                origin: self.origin(*source_ordinal),
                width_radius: self.coordinate(width_radius, *source_ordinal)?,
                height_radius: self.coordinate(height_radius, *source_ordinal)?,
                start_angle: self.angle(start_angle, *source_ordinal)?,
                sweep_angle: self.angle(sweep_angle, *source_ordinal)?,
            },
        })
    }
}
