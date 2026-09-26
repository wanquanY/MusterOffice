//! Author shape outlines in local centered coordinates. Elliptic arcs use cubic
//! Hermite interpolation with a certified coordinate remainder r*h^4/384.
use crate::{CompileError, cancel, interval::Interval as I, trig};
use mo_geometry::{Fixed, PathCommand as C, Point};
use mo_presentation_model::{Geometry, PathCommand as Author, Size};
use std::collections::BTreeMap;

const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
pub(crate) struct Outline {
    pub commands: Vec<C>,
    pub error: Point,
    pub curve_segments: u32,
}
#[derive(Default)]
pub(crate) struct ArcTable(BTreeMap<i64, [I; 2]>);
impl ArcTable {
    fn get(&mut self, angle: i64, check: &dyn Fn() -> bool) -> Result<[I; 2], CompileError> {
        let angle = angle.rem_euclid(trig::TURN);
        if let Some(v) = self.0.get(&angle) {
            return Ok(v.clone());
        }
        let v = trig::cos_sin(angle, check)?;
        self.0.insert(angle, v.clone());
        Ok(v)
    }
}
impl Outline {
    fn point(&mut self, p: [I; 2]) -> Result<Point, CompileError> {
        let (x, ex) = p[0].q32()?;
        let (y, ey) = p[1].q32()?;
        self.error.x = self.error.x.max(ex);
        self.error.y = self.error.y.max(ey);
        Ok(Point { x, y })
    }
    fn move_to(&mut self, p: [I; 2]) -> Result<(), CompileError> {
        let to = self.point(p)?;
        self.commands.push(C::Move { to });
        Ok(())
    }
    fn line_to(&mut self, p: [I; 2]) -> Result<(), CompileError> {
        let to = self.point(p)?;
        self.commands.push(C::Line { to });
        Ok(())
    }
    fn arc(
        &mut self,
        center: [I; 2],
        radii: [I; 2],
        start: i64,
        n: u32,
        table: &mut ArcTable,
        check: &dyn Fn() -> bool,
    ) -> Result<(), CompileError> {
        let step = (trig::TURN / 4) / i64::from(n);
        let tangent = trig::radians(step).divide(3);
        for j in 0..n {
            cancel(check)?;
            let [c0, s0] = table.get(start + i64::from(j) * step, check)?;
            let [c1, s1] = table.get(start + i64::from(j + 1) * step, check)?;
            let control1 = self.point([
                center[0].add(&radii[0].mul(&c0.sub(&tangent.mul(&s0)))),
                center[1].add(&radii[1].mul(&s0.add(&tangent.mul(&c0)))),
            ])?;
            let control2 = self.point([
                center[0].add(&radii[0].mul(&c1.add(&tangent.mul(&s1)))),
                center[1].add(&radii[1].mul(&s1.sub(&tangent.mul(&c1)))),
            ])?;
            let to = self.point([
                center[0].add(&radii[0].mul(&c1)),
                center[1].add(&radii[1].mul(&s1)),
            ])?;
            self.commands.push(C::Cubic {
                control1,
                control2,
                to,
            });
        }
        self.curve_segments += n;
        Ok(())
    }
}
pub(crate) fn curve_remainder(radii: &[I; 2], n: u32) -> [I; 2] {
    let h = trig::radians((trig::TURN / 4) / i64::from(n));
    let square = h.mul(&h);
    let fourth = square.mul(&square).divide(384);
    [radii[0].mul(&fourth), radii[1].mul(&fourth)]
}
pub(crate) fn radii(geometry: &Geometry, size: Size) -> Option<[I; 2]> {
    match geometry {
        Geometry::Ellipse => Some([
            I::integer(size.width.get()).divide(2),
            I::integer(size.height.get()).divide(2),
        ]),
        Geometry::RoundRectangle { radius } if radius.get() > 0 => {
            Some([I::integer(radius.get()), I::integer(radius.get())])
        }
        _ => None,
    }
}
pub(crate) fn outline(
    geometry: &Geometry,
    size: Size,
    anchor: Point,
    n: u32,
    table: &mut ArcTable,
    check: &dyn Fn() -> bool,
) -> Result<Outline, CompileError> {
    cancel(check)?;
    let mut out = Outline {
        commands: vec![],
        error: ZERO,
        curve_segments: 0,
    };
    let w = I::integer(size.width.get()).divide(2);
    let h = I::integer(size.height.get()).divide(2);
    match geometry {
        Geometry::Path { commands, .. } => {
            let point = |p: mo_presentation_model::Point| -> Result<Point, CompileError> {
                Ok(Point {
                    x: Fixed::emu(p.x)
                        .checked_sub(anchor.x)
                        .map_err(|_| CompileError::Range)?,
                    y: Fixed::emu(p.y)
                        .checked_sub(anchor.y)
                        .map_err(|_| CompileError::Range)?,
                })
            };
            for command in commands {
                cancel(check)?;
                out.commands.push(match command {
                    Author::Move { to } => C::Move { to: point(*to)? },
                    Author::Line { to } => C::Line { to: point(*to)? },
                    Author::Quadratic { control, to } => C::Quadratic {
                        control: point(*control)?,
                        to: point(*to)?,
                    },
                    Author::Cubic {
                        control1,
                        control2,
                        to,
                    } => C::Cubic {
                        control1: point(*control1)?,
                        control2: point(*control2)?,
                        to: point(*to)?,
                    },
                    Author::Close => C::Close,
                });
            }
        }
        Geometry::Ellipse => {
            let radii = [w.clone(), h.clone()];
            out.move_to([w.clone(), I::integer(0)])?;
            for quadrant in 0..4 {
                out.arc(
                    [I::integer(0), I::integer(0)],
                    radii.clone(),
                    quadrant * (trig::TURN / 4),
                    n,
                    table,
                    check,
                )?;
            }
            out.commands.push(C::Close);
        }
        Geometry::RoundRectangle { radius } if radius.get() > 0 => {
            let r = I::integer(radius.get());
            let a = w.sub(&r);
            let b = h.sub(&r);
            out.move_to([a.clone(), h.neg()])?;
            out.arc(
                [a.clone(), b.neg()],
                [r.clone(), r.clone()],
                3 * (trig::TURN / 4),
                n,
                table,
                check,
            )?;
            out.line_to([w.clone(), b.clone()])?;
            out.arc(
                [a.clone(), b.clone()],
                [r.clone(), r.clone()],
                0,
                n,
                table,
                check,
            )?;
            out.line_to([a.neg(), h.clone()])?;
            out.arc(
                [a.neg(), b.clone()],
                [r.clone(), r.clone()],
                trig::TURN / 4,
                n,
                table,
                check,
            )?;
            out.line_to([w.neg(), b.neg()])?;
            out.arc(
                [a.neg(), b.neg()],
                [r.clone(), r],
                trig::TURN / 2,
                n,
                table,
                check,
            )?;
            out.commands.push(C::Close);
        }
        Geometry::Rectangle | Geometry::RoundRectangle { .. } => {
            out.move_to([w.neg(), h.neg()])?;
            out.line_to([w.clone(), h.neg()])?;
            out.line_to([w, h.clone()])?;
            out.line_to([I::integer(size.width.get()).divide(2).neg(), h])?;
            out.commands.push(C::Close);
        }
    }
    if let Some(r) = radii(geometry, size) {
        let error = curve_remainder(&r, n);
        out.error.x = out
            .error
            .x
            .checked_add(error[0].upper_q32()?)
            .map_err(|_| CompileError::Range)?;
        out.error.y = out
            .error
            .y
            .checked_add(error[1].upper_q32()?)
            .map_err(|_| CompileError::Range)?;
    }
    Ok(out)
}
