use super::{CompiledNativePath, NativePathError, NativePathIssue, NativePathSpan, Work, math};
use crate::{hermite_arc, interval::Interval as I, trig};
use mo_geometry::{Fixed, PathCommand as C, Point};
use mo_presentation_source::source::geometry::evaluate::{
    EvaluatedCommand as E, EvaluatedPath, EvaluatedPoint,
};
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
struct Builder<'a, 'b> {
    commands: Vec<C>,
    scale: [I; 2],
    numeric: Point,
    curve: Point,
    segments: u32,
    tolerance: Fixed,
    work: &'a mut Work<'b>,
}
impl Builder<'_, '_> {
    fn point(&mut self, p: &[I; 2]) -> Result<Point, NativePathError> {
        self.work.step()?;
        let (x, ex) = p[0]
            .mul(&self.scale[0])
            .q32()
            .map_err(|_| self.work.numeric())?;
        let (y, ey) = p[1]
            .mul(&self.scale[1])
            .q32()
            .map_err(|_| self.work.numeric())?;
        self.numeric.x = self.numeric.x.max(ex);
        self.numeric.y = self.numeric.y.max(ey);
        Ok(Point { x, y })
    }
    fn push(&mut self, c: C) -> Result<(), NativePathError> {
        self.work.step()?;
        self.work.commands = self
            .work
            .commands
            .checked_add(1)
            .ok_or(NativePathError::Limit("commands"))?;
        if self.work.commands > self.work.limits.max_commands {
            return Err(NativePathError::Limit("commands"));
        }
        self.commands.push(c);
        Ok(())
    }
    fn input(&self, p: &EvaluatedPoint) -> Result<[I; 2], NativePathError> {
        Ok([
            I::binary64(p.x).map_err(|_| self.work.numeric())?,
            I::binary64(p.y).map_err(|_| self.work.numeric())?,
        ])
    }
    fn arc(
        &mut self,
        current: &[I; 2],
        radii: [f64; 2],
        start: f64,
        sweep: f64,
    ) -> Result<[I; 2], NativePathError> {
        if [radii[0], radii[1], start, sweep]
            .iter()
            .any(|x| !x.is_finite())
        {
            return Err(self.work.numeric());
        }
        if radii.iter().any(|x| *x < 0.0) {
            return Err(self.work.issue(NativePathIssue::NegativeRadius));
        }
        // A point ellipse has a unique collapsed locus for every angle. Native
        // rounded-rectangle definitions use it for a zero adjustment corner.
        if sweep == 0.0 || radii == [0.0, 0.0] {
            return Ok(current.clone());
        }
        if radii.contains(&0.0) {
            return Err(self.work.issue(NativePathIssue::DegenerateArcRadius));
        }
        if sweep.abs() > f64::from(self.work.limits.max_arc_turns) * trig::TURN as f64 {
            return Err(NativePathError::Limit("arc turns"));
        }
        let radii = [
            I::binary64(radii[0]).map_err(|_| self.work.numeric())?,
            I::binary64(radii[1]).map_err(|_| self.work.numeric())?,
        ];
        let start = math::normalize_start(I::binary64(start).map_err(|_| self.work.numeric())?);
        let end = start.add(&I::binary64(sweep).map_err(|_| self.work.numeric())?);
        let a = math::parametric(&start, &radii, self.work)?;
        let b = math::parametric(&end, &radii, self.work)?;
        let delta = b.sub(&a);
        let mut n = 1u32;
        let mut h;
        let mut error;
        loop {
            self.work.step()?;
            h = delta.divide(i64::from(n));
            error = hermite_arc::error(
                &[radii[0].mul(&self.scale[0]), radii[1].mul(&self.scale[1])],
                &h,
            )
            .map_err(|_| self.work.numeric())?;
            if crate::coordinate_budget::curve_fits_local(error, self.tolerance) {
                break;
            }
            n = n
                .checked_mul(2)
                .ok_or(NativePathError::Limit("arc segments"))?;
            if n > self.work.limits.max_arc_segments {
                return Err(NativePathError::Limit("arc segments"));
            }
        }
        self.work.segments = self
            .work
            .segments
            .checked_add(n)
            .ok_or(NativePathError::Limit("arc segments"))?;
        if self.work.segments > self.work.limits.max_arc_segments {
            return Err(NativePathError::Limit("arc segments"));
        }
        // The start angle is locked to the existing pen; there is no hidden
        // line from the pen to a separately guessed ellipse start point.
        let mut cs = math::cos_sin(&a, self.work)?;
        let center = [
            current[0].sub(&radii[0].mul(&cs[0])),
            current[1].sub(&radii[1].mul(&cs[1])),
        ];
        let mut to = current.clone();
        for j in 0..n {
            self.work.step()?;
            let angle = if j + 1 == n {
                b.clone()
            } else {
                a.add(
                    &delta
                        .mul(&I::integer(i64::from(j + 1)))
                        .divide(i64::from(n)),
                )
            };
            let next = math::cos_sin(&angle, self.work)?;
            let [c1, c2, end] = hermite_arc::cubic(&center, &radii, &cs, &next, &h);
            let control1 = self.point(&c1)?;
            let control2 = self.point(&c2)?;
            to = end;
            let p = self.point(&to)?;
            self.push(C::Cubic {
                control1,
                control2,
                to: p,
            })?;
            cs = next;
        }
        self.curve.x = self.curve.x.max(error[0]);
        self.curve.y = self.curve.y.max(error[1]);
        self.segments += n;
        Ok(to)
    }
}
pub(super) fn compile(
    path: &EvaluatedPath,
    scale: [I; 2],
    tolerance: Fixed,
    work: &mut Work<'_>,
) -> Result<CompiledNativePath, NativePathError> {
    let mut b = Builder {
        commands: Vec::new(),
        scale,
        numeric: ZERO,
        curve: ZERO,
        segments: 0,
        tolerance,
        work,
    };
    let mut current = [I::integer(0), I::integer(0)];
    let mut start = current.clone();
    let mut needs_move = true;
    let mut spans = Vec::new();
    for command in &path.commands {
        let ordinal = match command {
            E::Move { origin, .. }
            | E::Line { origin, .. }
            | E::Quadratic { origin, .. }
            | E::Cubic { origin, .. }
            | E::Arc { origin, .. }
            | E::Close { origin } => *origin,
        };
        b.work.origin = ordinal;
        b.work.step()?;
        let first = b.commands.len() as u32;
        if needs_move && !matches!(command, E::Move { .. }) {
            let to = b.point(&current)?;
            b.push(C::Move { to })?;
            start = current.clone();
            needs_move = false;
        }
        match command {
            E::Move { to, .. } => {
                current = b.input(to)?;
                start = current.clone();
                let to = b.point(&current)?;
                b.push(C::Move { to })?;
                needs_move = false;
            }
            E::Line { to, .. } => {
                current = b.input(to)?;
                let to = b.point(&current)?;
                b.push(C::Line { to })?;
            }
            E::Quadratic { control, to, .. } => {
                let c = b.input(control)?;
                let control = b.point(&c)?;
                current = b.input(to)?;
                let to = b.point(&current)?;
                b.push(C::Quadratic { control, to })?;
            }
            E::Cubic {
                control1,
                control2,
                to,
                ..
            } => {
                let c1 = b.input(control1)?;
                let c2 = b.input(control2)?;
                let control1 = b.point(&c1)?;
                let control2 = b.point(&c2)?;
                current = b.input(to)?;
                let to = b.point(&current)?;
                b.push(C::Cubic {
                    control1,
                    control2,
                    to,
                })?;
            }
            E::Arc {
                width_radius,
                height_radius,
                start_angle,
                sweep_angle,
                ..
            } => {
                current = b.arc(
                    &current,
                    [*width_radius, *height_radius],
                    *start_angle,
                    *sweep_angle,
                )?;
            }
            E::Close { .. } => {
                b.push(C::Close)?;
                current = start.clone();
                needs_move = true;
            }
        }
        spans.push(NativePathSpan {
            origin: ordinal,
            first_command: first,
            command_count: b.commands.len() as u32 - first,
        });
    }
    let error = Point {
        x: b.numeric
            .x
            .checked_add(b.curve.x)
            .map_err(|_| b.work.numeric())?,
        y: b.numeric
            .y
            .checked_add(b.curve.y)
            .map_err(|_| b.work.numeric())?,
    };
    if error.x > tolerance || error.y > tolerance {
        return Err(b.work.precision());
    }
    Ok(CompiledNativePath {
        origin: path.origin,
        commands: b.commands,
        source_map: spans,
        fill: path.fill,
        stroke: path.stroke,
        extrusion_ok: path.extrusion_ok,
        numeric_error_bound: b.numeric,
        curve_error_bound: b.curve,
        coordinate_error_bound: error,
        arc_segments: b.segments,
    })
}
