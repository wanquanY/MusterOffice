use crate::*;
#[derive(Debug, Clone, Copy)]
pub struct BoundsBudget {
    remaining: u32,
    pub visited: u32,
}
impl BoundsBudget {
    pub fn new(maximum: u32) -> Self {
        Self {
            remaining: maximum,
            visited: 0,
        }
    }
    fn spend(&mut self, check: &dyn Fn() -> bool) -> Result<(), GeometryError> {
        if check() {
            return Err(GeometryError::Cancelled);
        }
        if self.remaining == 0 {
            return Err(GeometryError::Limit("curve bounds work"));
        }
        self.remaining -= 1;
        self.visited += 1;
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Interval {
    lo: i128,
    hi: i128,
}
impl Interval {
    fn exact(v: Fixed) -> Self {
        Self {
            lo: v.raw(),
            hi: v.raw(),
        }
    }
    fn half_sum(a: i128, b: i128, up: bool) -> i128 {
        // Euclidean division makes negative and odd endpoints outward-rounded;
        // splitting the sum first also handles the entire i128 input domain.
        let q = a.div_euclid(2) + b.div_euclid(2);
        let r = a.rem_euclid(2) + b.rem_euclid(2);
        q + r / 2 + i128::from(up && r % 2 != 0)
    }
    fn midpoint(self, other: Self) -> Self {
        Self {
            lo: Self::half_sum(self.lo, other.lo, false),
            hi: Self::half_sum(self.hi, other.hi, true),
        }
    }
}
#[derive(Clone, Copy)]
struct Pair {
    x: Interval,
    y: Interval,
}
impl Pair {
    fn point(p: Point) -> Self {
        Self {
            x: Interval::exact(p.x),
            y: Interval::exact(p.y),
        }
    }
    fn midpoint(self, other: Self) -> Self {
        Self {
            x: self.x.midpoint(other.x),
            y: self.y.midpoint(other.y),
        }
    }
    fn hull(self) -> Rect {
        Rect {
            min: Point {
                x: Fixed::from_raw(self.x.lo),
                y: Fixed::from_raw(self.y.lo),
            },
            max: Point {
                x: Fixed::from_raw(self.x.hi),
                y: Fixed::from_raw(self.y.hi),
            },
        }
    }
    fn witness(self) -> Rect {
        Rect {
            min: Point {
                x: Fixed::from_raw(self.x.hi),
                y: Fixed::from_raw(self.y.hi),
            },
            max: Point {
                x: Fixed::from_raw(self.x.lo),
                y: Fixed::from_raw(self.y.lo),
            },
        }
    }
}
#[derive(Clone, Copy)]
struct Curve {
    p: [Pair; 4],
    degree: usize,
    depth: u8,
}
impl Curve {
    fn split(self) -> (Self, Self) {
        let mut work = self.p;
        let mut left = self;
        let mut right = self;
        for level in 1..=self.degree {
            for i in 0..=self.degree - level {
                work[i] = work[i].midpoint(work[i + 1]);
            }
            left.p[level] = work[0];
            right.p[self.degree - level] = work[self.degree - level];
        }
        left.depth += 1;
        right.depth += 1;
        (left, right)
    }
    fn hull(self) -> Rect {
        self.p[1..=self.degree]
            .iter()
            .fold(self.p[0].hull(), |a, p| a.union(p.hull()))
    }
}
fn fits(hull: Rect, witness: Rect, tolerance: Fixed) -> bool {
    [
        (witness.min.x, hull.min.x),
        (witness.min.y, hull.min.y),
        (hull.max.x, witness.max.x),
        (hull.max.y, witness.max.y),
    ]
    .iter()
    .all(|(a, b)| {
        a.raw()
            .checked_sub(b.raw())
            .is_some_and(|v| v <= tolerance.raw())
    })
}
fn include(target: &mut Option<Rect>, r: Rect) {
    *target = Some(target.map_or(r, |b| b.union(r)));
}
fn bound_curve(
    points: &[Point],
    tolerance: Fixed,
    budget: &mut BoundsBudget,
    check: &dyn Fn() -> bool,
    stack: &mut Vec<Curve>,
) -> Result<Rect, GeometryError> {
    let mut p = [Pair::point(points[0]); 4];
    for (to, &from) in p.iter_mut().zip(points) {
        *to = Pair::point(from);
    }
    let curve = Curve {
        p,
        degree: points.len() - 1,
        depth: 0,
    };
    let mut witness = p[0].witness().union(p[curve.degree].witness());
    let mut accepted = None;
    stack.clear();
    stack.push(curve);
    while let Some(c) = stack.pop() {
        budget.spend(check)?;
        witness = witness
            .union(c.p[0].witness())
            .union(c.p[c.degree].witness());
        let hull = c.hull();
        if fits(hull, witness, tolerance) {
            include(&mut accepted, hull);
            continue;
        }
        if c.depth >= 64 {
            return Err(GeometryError::Limit("curve bounds subdivision depth"));
        }
        let (left, right) = c.split();
        stack.push(right);
        stack.push(left);
    }
    Ok(accepted.expect("at least one leaf"))
}
/// Bounds the locus of line/quadratic/cubic/closing segments, not filled ink,
/// strokes or effects. Each edge is outward with excess <= tolerance in Q32.
/// Move-only paths have no locus. Open contours are permitted for later strokes.
pub fn path_bounds(
    commands: &[PathCommand],
    tolerance: Fixed,
    budget: &mut BoundsBudget,
    check: &dyn Fn() -> bool,
) -> Result<Option<Rect>, GeometryError> {
    if tolerance.raw() < 256 {
        return Err(GeometryError::Invalid(
            "bounds tolerance must be at least 256 Q32 units",
        ));
    }
    let mut current = None;
    let mut start = None;
    let mut result = None;
    let mut stack = Vec::with_capacity(65);
    for command in commands {
        budget.spend(check)?;
        use PathCommand::*;
        if let Move { to } = *command {
            current = Some(to);
            start = Some(to);
            continue;
        }
        let from = current.ok_or(GeometryError::Invalid("path segment without move"))?;
        let mut points = [from; 4];
        let count = match *command {
            Move { .. } => unreachable!(),
            Line { to } => {
                points[1] = to;
                2
            }
            Quadratic { control, to } => {
                points[1] = control;
                points[2] = to;
                3
            }
            Cubic {
                control1,
                control2,
                to,
            } => {
                points[1] = control1;
                points[2] = control2;
                points[3] = to;
                4
            }
            Close => {
                points[1] = start.ok_or(GeometryError::Invalid("path close without move"))?;
                2
            }
        };
        let rect = bound_curve(&points[..count], tolerance, budget, check, &mut stack)?;
        include(&mut result, rect);
        current = Some(points[count - 1]);
        if matches!(command, Close) {
            current = None;
            start = None;
        }
    }
    if check() {
        return Err(GeometryError::Cancelled);
    }
    Ok(result)
}
