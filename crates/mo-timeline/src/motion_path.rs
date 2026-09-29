//! Distance-paced connected native paths. Compilation owns all subdivision;
//! frame sampling searches a retained cumulative-length table in O(log N).
mod numeric;
use crate::{
    ExactMotion, MotionPath, MotionPoint, MotionSegment, TimelineError, TimelineLimits, cancel,
    exact::Ratio,
};
use num_bigint::BigInt;
use numeric::{Point, TOLERANCE, UNIT};

#[derive(Debug, Clone)]
struct Edge {
    from: Point,
    to: Point,
    begin: i128,
    end: i128,
}
#[derive(Debug, Clone)]
pub(crate) struct PathPlan {
    edges: Vec<Edge>,
    first: MotionPoint,
    last: MotionPoint,
    total: i128,
}

/// Shared across the entire timeline, not reset for each object or segment.
pub(crate) struct Budget {
    segments: usize,
    vertices: usize,
    steps: usize,
    limits: TimelineLimits,
}
impl Budget {
    pub fn new(limits: TimelineLimits) -> Result<Self, TimelineError> {
        // These caps also bound cumulative Q64 subdivision rounding. A caller
        // may tighten resource limits, but cannot weaken the quality profile.
        if limits.max_motion_vertices > 1_048_576 || limits.max_motion_segments > 65_536 {
            return Err(TimelineError::Limit("motion path policy"));
        }
        Ok(Self {
            segments: 0,
            vertices: 0,
            steps: 0,
            limits,
        })
    }
    fn step(&mut self, check: &dyn Fn() -> bool) -> Result<(), TimelineError> {
        cancel(check)?;
        self.steps += 1;
        if self.steps > self.limits.max_motion_steps {
            return Err(TimelineError::Limit("motion path subdivision work"));
        }
        Ok(())
    }
    fn vertex(&mut self) -> Result<(), TimelineError> {
        self.vertices += 1;
        if self.vertices > self.limits.max_motion_vertices {
            return Err(TimelineError::Limit("motion path vertices"));
        }
        Ok(())
    }
}
impl PathPlan {
    pub fn compile(
        path: &MotionPath,
        budget: &mut Budget,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TimelineError> {
        cancel(check)?;
        if path.segments.is_empty() {
            return Err(crate::invalid(None, "motion path requires a segment"));
        }
        if path.segments.len() > 4096 {
            return Err(TimelineError::Limit("motion path segments per behavior"));
        }
        budget.segments = budget.segments.saturating_add(path.segments.len());
        if budget.segments > budget.limits.max_motion_segments {
            return Err(TimelineError::Limit("motion path segments"));
        }
        let mut result = Self {
            edges: vec![],
            first: path.from.clone(),
            last: path.from.clone(),
            total: 0,
        };
        let mut current = Point::from_source(&path.from);
        let first = current;
        // Accepted leaf gaps sum to at most this budget per path. The control
        // polygon is an arc-length upper bound; the chord is a lower bound.
        let gap_budget = TOLERANCE / path.segments.len() as i128;
        for segment in &path.segments {
            budget.step(check)?;
            let to = match segment {
                MotionSegment::Line { to } | MotionSegment::Cubic { to, .. } => to,
                MotionSegment::Close => &path.from,
            };
            let end = Point::from_source(to);
            if let MotionSegment::Cubic {
                control1, control2, ..
            } = segment
            {
                result.curve(
                    [
                        current,
                        Point::from_source(control1),
                        Point::from_source(control2),
                        end,
                    ],
                    gap_budget,
                    budget,
                    check,
                )?;
            } else {
                result.edge(
                    current,
                    if matches!(segment, MotionSegment::Close) {
                        first
                    } else {
                        end
                    },
                    budget,
                )?;
            }
            current = end;
            result.last = to.clone();
        }
        Ok(result)
    }
    fn edge(&mut self, from: Point, to: Point, budget: &mut Budget) -> Result<(), TimelineError> {
        budget.vertex()?;
        let length = from.distance(to).0;
        if length > 0 {
            let end = self
                .total
                .checked_add(length)
                .ok_or(TimelineError::Limit("motion path length"))?;
            self.edges.push(Edge {
                from,
                to,
                begin: self.total,
                end,
            });
            self.total = end;
        }
        Ok(())
    }
    fn curve(
        &mut self,
        points: [Point; 4],
        gap: i128,
        budget: &mut Budget,
        check: &dyn Fn() -> bool,
    ) -> Result<(), TimelineError> {
        let mut stack = vec![(points, gap, 0u8)];
        while let Some((p, gap, depth)) = stack.pop() {
            budget.step(check)?;
            let chord = p[0].distance(p[3]).0;
            let polygon: i128 = p.windows(2).map(|pair| pair[0].distance(pair[1]).1).sum();
            if polygon - chord <= gap
                && p[1].near_segment(p[0], p[3])
                && p[2].near_segment(p[0], p[3])
            {
                self.edge(p[0], p[3], budget)?;
            } else {
                if depth == 32 || gap < 2 {
                    return Err(TimelineError::Limit("motion path precision"));
                }
                let (left, right) = numeric::split(p);
                stack.push((right, gap / 2, depth + 1));
                stack.push((left, gap / 2, depth + 1));
            }
        }
        Ok(())
    }
    pub fn sample(&self, progress: &Ratio, bits: u64) -> Result<ExactMotion, TimelineError> {
        let endpoint = |p: &MotionPoint| {
            Ok(ExactMotion {
                x: p.x.ratio(bits)?.wire(),
                y: p.y.ratio(bits)?.wire(),
            })
        };
        if !progress.positive() {
            return endpoint(&self.first);
        }
        if progress.cmp(&Ratio::integer(1)).is_ge() {
            return endpoint(&self.last);
        }
        if self.total == 0 {
            return endpoint(&self.first);
        }
        let distance = progress.mul(&Ratio::new(self.total.into(), 1.into(), bits)?, bits)?;
        let index = self
            .edges
            .partition_point(|edge| BigInt::from(edge.end) * &distance.d < distance.n);
        let edge = &self.edges[index.min(self.edges.len() - 1)];
        let fraction = distance
            .sub(&Ratio::new(edge.begin.into(), 1.into(), bits)?, bits)?
            .div(
                &Ratio::new((edge.end - edge.begin).into(), 1.into(), bits)?,
                bits,
            )?;
        let axis = |a: i128, b: i128| {
            Ok::<_, TimelineError>(
                Ratio::new(a.into(), UNIT.into(), bits)?
                    .add(
                        &Ratio::new((b - a).into(), UNIT.into(), bits)?.mul(&fraction, bits)?,
                        bits,
                    )?
                    .wire(),
            )
        };
        Ok(ExactMotion {
            x: axis(edge.from.0, edge.to.0)?,
            y: axis(edge.from.1, edge.to.1)?,
        })
    }
}

pub(crate) fn compile(
    timeline: &crate::Timeline,
    limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Option<PathPlan>>, TimelineError> {
    let mut budget = Budget::new(limits)?;
    timeline
        .nodes
        .iter()
        .map(|node| {
            cancel(check)?;
            match &node.effect {
                crate::Effect::MotionPath { path, .. } => {
                    PathPlan::compile(path, &mut budget, check).map(Some)
                }
                _ => Ok(None),
            }
        })
        .collect()
}
