//! Exact-decimal circular chart sectors -> shared Q32 Draw IR paths. This is
//! geometry only: chart data authority, format defaults, labels, styles and
//! native chart serialization are separate responsibilities.
#[cfg(test)]
mod tests;
mod types;
pub use types::*;

use crate::{hermite_arc, interval::Interval as I, interval_extended::floor_ratio, trig};
use mo_charts::sectors;
use mo_geometry::{Fixed, PathCommand as C, Point};
use num_bigint::BigInt;

const TURN: i128 = 1i128 << 32;
const MAX_COORD: i128 = 27_273_042_316_900i128 << 32;

fn charge(value: &mut u32, n: u32, max: u32, name: &'static str) -> Result<(), ChartGeometryError> {
    *value = value
        .checked_add(n)
        .filter(|v| *v <= max)
        .ok_or(ChartGeometryError::Limit(name))?;
    Ok(())
}
fn tau() -> I {
    I::raw(BigInt::from(trig::PI_LO), BigInt::from(trig::PI_LO + 1)).mul(&I::integer(2))
}
struct Builder<'a> {
    request: &'a ChartGeometryRequest,
    limits: ChartGeometryLimits,
    check: &'a dyn Fn() -> bool,
    work: ChartGeometryWork,
    numeric: Fixed,
    curve: Fixed,
    curve_tolerance: Fixed,
    commands: Vec<C>,
}
impl Builder<'_> {
    fn step(&mut self, n: u32) -> Result<(), ChartGeometryError> {
        if (self.check)() {
            return Err(ChartGeometryError::Cancelled);
        }
        charge(&mut self.work.steps, n, self.limits.max_steps, "work")
    }
    /// Sampling subdivides a Q32 turn by powers of two, so every Q96 angle is
    /// exact. Reduce before converting to native angle units: all cardinal and
    /// shared endpoints remain bit-identical, even across a wrap or reversal.
    fn cos_sin(&mut self, turn: &I) -> Result<[I; 2], ChartGeometryError> {
        self.step(24)?;
        if turn.lo != turn.hi {
            return Err(ChartGeometryError::Precision);
        }
        let quarter = BigInt::from(1) << 94usize;
        let q = floor_ratio(&turn.lo, &quarter);
        let remainder = &turn.lo - &q * &quarter;
        let swap = remainder > (&quarter >> 1usize);
        let reduced = if swap { quarter - remainder } else { remainder };
        let quadrant = (i64::try_from(q).map_err(|_| ChartGeometryError::Range)? - 1).rem_euclid(4);
        trig::reduced_cos_sin(
            quadrant,
            swap,
            I::raw(reduced.clone(), reduced).mul(&I::integer(trig::TURN)),
            self.check,
        )
        .map_err(|_| ChartGeometryError::Cancelled)
    }
    fn point(&mut self, p: &[I; 2]) -> Result<Point, ChartGeometryError> {
        self.step(1)?;
        let (x, ex) = p[0].q32().map_err(|_| ChartGeometryError::Range)?;
        let (y, ey) = p[1].q32().map_err(|_| ChartGeometryError::Range)?;
        self.numeric = self.numeric.max(ex).max(ey);
        Ok(Point { x, y })
    }
    fn at(&mut self, radius: Fixed, angle: Fixed) -> Result<Point, ChartGeometryError> {
        let [c, s] = self.cos_sin(&I::fixed(angle))?;
        self.point(&[
            I::fixed(self.request.center.x).add(&I::fixed(radius).mul(&c)),
            I::fixed(self.request.center.y).add(&I::fixed(radius).mul(&s)),
        ])
    }
    fn push(&mut self, command: C) -> Result<(), ChartGeometryError> {
        self.step(1)?;
        charge(
            &mut self.work.commands,
            1,
            self.limits.max_commands,
            "commands",
        )?;
        self.commands.push(command);
        Ok(())
    }
    fn arc(&mut self, radius: Fixed, start: Fixed, end: Fixed) -> Result<(), ChartGeometryError> {
        let delta = I::fixed(end).sub(&I::fixed(start));
        let radians = delta.mul(&tau());
        let radii = [I::fixed(radius), I::fixed(radius)];
        let center = [
            I::fixed(self.request.center.x),
            I::fixed(self.request.center.y),
        ];
        let mut n = 1u32;
        let (h, error) = loop {
            self.step(1)?;
            if n > self
                .limits
                .max_arc_segments
                .saturating_sub(self.work.arc_segments)
            {
                return Err(ChartGeometryError::Limit("arc segments"));
            }
            let h = radians.divide(i64::from(n));
            let error = hermite_arc::error(&radii, &h).map_err(|_| ChartGeometryError::Range)?;
            if error.iter().all(|bound| *bound <= self.curve_tolerance) {
                break (h, error[0].max(error[1]));
            }
            n = n
                .checked_mul(2)
                .ok_or(ChartGeometryError::Limit("arc segments"))?;
        };
        charge(
            &mut self.work.arc_segments,
            n,
            self.limits.max_arc_segments,
            "arc segments",
        )?;
        self.curve = self.curve.max(error);
        let mut previous = self.cos_sin(&I::fixed(start))?;
        for j in 1..=n {
            let angle =
                I::fixed(start).add(&delta.mul(&I::integer(i64::from(j))).divide(i64::from(n)));
            let next = self.cos_sin(&angle)?;
            let [c1, c2, to] = hermite_arc::cubic(&center, &radii, &previous, &next, &h);
            let control1 = self.point(&c1)?;
            let control2 = self.point(&c2)?;
            let to = self.point(&to)?;
            self.push(C::Cubic {
                control1,
                control2,
                to,
            })?;
            previous = next;
        }
        Ok(())
    }
    fn sector(
        &mut self,
        sector: &sectors::Sector,
        angular: Fixed,
    ) -> Result<ChartSectorPath, ChartGeometryError> {
        self.step(1)?;
        self.numeric = Fixed::ZERO;
        self.curve = Fixed::ZERO;
        let before = self.work.arc_segments;
        let start = sector.start_turn;
        let end = sector.end_turn;
        let empty = start == end;
        if !empty {
            let full = (end.raw() - start.raw()).abs() == TURN;
            let outer = self.request.outer_radius;
            let inner = self.request.inner_radius;
            let to = self.at(outer, start)?;
            self.push(C::Move { to })?;
            self.arc(outer, start, end)?;
            if full {
                self.push(C::Close)?;
            }
            if inner.raw() != 0 {
                let to = self.at(inner, end)?;
                self.push(if full { C::Move { to } } else { C::Line { to } })?;
                self.arc(inner, end, start)?;
                self.push(C::Close)?;
            } else if !full {
                self.push(C::Line {
                    to: self.request.center,
                })?;
                self.push(C::Close)?;
            }
        }
        let angular = if empty { Fixed::ZERO } else { angular };
        let total = self
            .numeric
            .raw()
            .checked_add(self.curve.raw())
            .and_then(|n| n.checked_add(angular.raw()))
            .ok_or(ChartGeometryError::Range)?;
        if total > self.request.coordinate_tolerance.raw() {
            return Err(ChartGeometryError::Precision);
        }
        Ok(ChartSectorPath {
            point_index: sector.point_index,
            commands: std::mem::take(&mut self.commands),
            numeric_error_bound: self.numeric,
            curve_error_bound: self.curve,
            angular_error_bound: angular,
            coordinate_error_bound: Fixed::from_raw(total),
            arc_segments: self.work.arc_segments - before,
        })
    }
}

/// Compute every sector with one request-wide geometry budget. Failures return
/// no partial geometry. Source chart admission must still resolve data/style/
/// layout semantics and compose these bounds with page/raster uncertainty.
pub fn compile(
    request: &ChartGeometryRequest,
    limits: ChartGeometryLimits,
    check: &dyn Fn() -> bool,
) -> Result<ChartGeometry, ChartGeometryError> {
    if check() {
        return Err(ChartGeometryError::Cancelled);
    }
    if request.coordinate_tolerance.raw() <= 0 {
        return Err(ChartGeometryError::Invalid("coordinate tolerance"));
    }
    if request.outer_radius.raw() <= 0
        || request.outer_radius.raw() > MAX_COORD
        || request.inner_radius.raw() < 0
        || request.inner_radius >= request.outer_radius
    {
        return Err(ChartGeometryError::Invalid("radii"));
    }
    if [request.center.x.raw(), request.center.y.raw()]
        .iter()
        .any(|n| !(-MAX_COORD..=MAX_COORD).contains(n))
    {
        return Err(ChartGeometryError::Invalid("center"));
    }
    let paths = u32::try_from(request.sectors.weights.len())
        .map_err(|_| ChartGeometryError::Limit("paths"))?;
    if paths > limits.max_paths {
        return Err(ChartGeometryError::Limit("paths"));
    }
    let layout = sectors::layout(&request.sectors, limits.sectors, check)?;
    if layout
        .sectors
        .iter()
        .any(|sector| sector.positive_below_resolution)
    {
        return Err(ChartGeometryError::Precision);
    }
    // |sin(a)-sin(b)| and |cos(a)-cos(b)| <= |a-b|. Interpolating
    // both endpoint angles never increases their maximum angular uncertainty.
    let angular = I::fixed(layout.endpoint_error_bound)
        .mul(&tau())
        .mul(&I::fixed(request.outer_radius))
        .upper_q32()
        .map_err(|_| ChartGeometryError::Range)?;
    let remaining = request.coordinate_tolerance.raw() - angular.raw();
    if remaining <= 1 && !layout.zero_total {
        return Err(ChartGeometryError::Precision);
    }
    let mut builder = Builder {
        request,
        limits,
        check,
        work: ChartGeometryWork {
            paths,
            ..Default::default()
        },
        numeric: Fixed::ZERO,
        curve: Fixed::ZERO,
        // Refine against the remaining error budget: angular allocation may
        // already consume most of the original coordinate tolerance.
        curve_tolerance: Fixed::from_raw(
            (request.coordinate_tolerance.raw() / 4)
                .max(1)
                .min((remaining / 2).max(1)),
        ),
        commands: Vec::new(),
    };
    let mut paths = Vec::with_capacity(layout.sectors.len());
    for sector in &layout.sectors {
        paths.push(builder.sector(sector, angular)?);
    }
    builder.step(1)?;
    Ok(ChartGeometry {
        profile: PROFILE.into(),
        layout,
        fill_rule: mo_raster::FillRule::Nonzero,
        paths,
        work: builder.work,
    })
}
