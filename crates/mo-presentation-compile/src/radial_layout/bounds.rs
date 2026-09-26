//! True path locus bounds with outward curve and upstream conversion errors.
use super::*;
use mo_geometry::{BoundsBudget, PathCommand, path_bounds};

pub(super) fn collect(
    paths: &[CompiledNativePath],
    tolerance: Fixed,
    budget: &mut BoundsBudget,
    check: &dyn Fn() -> bool,
) -> Result<([I; 4], RadialLayoutWork), RadialLayoutError> {
    let before = budget.visited;
    let mut edges: Option<[I; 4]> = None;
    let mut work = RadialLayoutWork {
        paths: 0,
        commands: 0,
        bounds_steps: 0,
        arc_segments: 0,
    };
    for path in paths {
        cancel(check)?;
        let errors = [path.coordinate_error_bound.x, path.coordinate_error_bound.y];
        if errors.iter().any(|e| e.raw() < 0) {
            return Err(RadialLayoutError::Invalid(
                "negative native path uncertainty",
            ));
        }
        let Some(rect) = path_bounds(&path.commands, tolerance, budget, check)? else {
            continue;
        };
        let curved = path
            .commands
            .iter()
            .any(|p| matches!(p, PathCommand::Quadratic { .. } | PathCommand::Cubic { .. }));
        let excess = if curved {
            I::fixed(tolerance)
        } else {
            I::integer(0)
        };
        let raw = [rect.min.x, rect.min.y, rect.max.x, rect.max.y];
        let values: [I; 4] = std::array::from_fn(|i| {
            let v = I::fixed(raw[i]);
            let e = I::fixed(errors[i % 2]);
            if i < 2 {
                I::raw(&v.lo - &e.hi, &v.hi + &excess.hi + &e.hi)
            } else {
                I::raw(&v.lo - &excess.hi - &e.hi, &v.hi + &e.hi)
            }
        });
        if let Some(existing) = &mut edges {
            for i in 0..4 {
                if i < 2 {
                    existing[i].lo = existing[i].lo.clone().min(values[i].lo.clone());
                    existing[i].hi = existing[i].hi.clone().min(values[i].hi.clone());
                } else {
                    existing[i].lo = existing[i].lo.clone().max(values[i].lo.clone());
                    existing[i].hi = existing[i].hi.clone().max(values[i].hi.clone());
                }
            }
        } else {
            edges = Some(values);
        }
        work.paths = work.paths.checked_add(1).ok_or(RadialLayoutError::Range)?;
        work.commands = work
            .commands
            .checked_add(u32::try_from(path.commands.len()).map_err(|_| RadialLayoutError::Range)?)
            .ok_or(RadialLayoutError::Range)?;
        work.arc_segments = work
            .arc_segments
            .checked_add(path.arc_segments)
            .ok_or(RadialLayoutError::Range)?;
    }
    work.bounds_steps = budget.visited - before;
    Ok((
        edges.ok_or(RadialLayoutError::Invalid(
            "gradient geometry has no path locus",
        ))?,
        work,
    ))
}
