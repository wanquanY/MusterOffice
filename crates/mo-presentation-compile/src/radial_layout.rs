//! Native DrawingML circle geometry before world placement and scalar shading.
//! Bounds and focus geometry are independently queryable; this is not a renderer.
mod bounds;
mod source;
mod types;
use crate::{
    interval::Interval as I, interval_extended::floor_ratio, native_paths::*,
    source_number::percentage_ratio,
};
use mo_geometry::{BoundsBudget, Fixed};
use mo_presentation_model::Size;
use mo_presentation_source::source::fill::{
    NativePathShade,
    resolve::{EffectiveFillRect, EffectiveGradientFill, EffectiveGradientShade},
};
use num_bigint::BigInt;
pub use source::{
    SourceRadialLayoutError, SourceRadialLayoutPlan, layout_source, layout_source_with_basis,
};
pub use types::*;

pub const PROFILE: &str = "drawingml-circle-path-bounds-q96-v1-draft";
pub const ANCHOR_FOCUS_PROFILE: &str = "drawingml-circle-anchor-focus-q96-v2-draft";
fn cancel(check: &dyn Fn() -> bool) -> Result<(), RadialLayoutError> {
    if check() {
        Err(RadialLayoutError::Cancelled)
    } else {
        Ok(())
    }
}
fn numeric(_: crate::CompileError) -> RadialLayoutError {
    RadialLayoutError::Range
}
fn estimate<const N: usize>(v: [I; N]) -> Result<RadialEstimate<N>, RadialLayoutError> {
    let mut values = [Fixed::ZERO; N];
    let mut errors = values;
    for (i, v) in v.iter().enumerate() {
        (values[i], errors[i]) = v.q32().map_err(numeric)?;
    }
    Ok(RadialEstimate { values, errors })
}
fn ratio(n: BigInt, d: BigInt) -> I {
    let (n, d) = if d < BigInt::from(0) {
        (-n, -d)
    } else {
        (n, d)
    };
    let n = n << 96usize;
    I::raw(floor_ratio(&n, &d), -floor_ratio(&(-n), &d))
}
fn percentages(r: &EffectiveFillRect) -> Result<[(BigInt, BigInt); 4], RadialLayoutError> {
    let values = [&r.left, &r.top, &r.right, &r.bottom].map(|v| {
        percentage_ratio(&v.value)
            .map_err(|_| RadialLayoutError::Invalid("percentage lexical limit or range"))
    });
    let [l, t, r, b] = values;
    Ok([l?, t?, r?, b?])
}
fn tile(bounds: &[I; 4], fill: &EffectiveFillRect) -> Result<[I; 4], RadialLayoutError> {
    let p = percentages(fill)?.map(|(n, d)| ratio(n, d));
    let ext = [bounds[2].sub(&bounds[0]), bounds[3].sub(&bounds[1])];
    let r = std::array::from_fn(|i| {
        if i < 2 {
            bounds[i].add(&ext[i].mul(&p[i]))
        } else {
            bounds[i].sub(&ext[i % 2].mul(&p[i]))
        }
    });
    let r: [I; 4] = r;
    if r[2].sub(&r[0]).lo <= BigInt::from(0) || r[3].sub(&r[1]).lo <= BigInt::from(0) {
        return Err(RadialLayoutError::Invalid(
            "radial tile must have certified positive extents",
        ));
    }
    Ok(r)
}
/// Bounded shared geometry work. Keep one bounds budget across all targets.
/// Compiled paths are trusted output of NativePathCompiler for the receiving
/// shape, before anchor rebasing. Stroke width/effects are intentionally absent.
pub fn layout_paths(
    fill: &EffectiveGradientFill,
    paths: &[CompiledNativePath],
    options: RadialLayoutOptions,
    budget: &mut BoundsBudget,
    check: &dyn Fn() -> bool,
) -> Result<NativeRadialLayout, RadialLayoutError> {
    layout_paths_with_basis(
        fill,
        paths,
        options,
        budget,
        CircleFocusBasis::CircumscribedSquare,
        check,
    )
}
pub fn layout_paths_with_basis(
    fill: &EffectiveGradientFill,
    paths: &[CompiledNativePath],
    options: RadialLayoutOptions,
    budget: &mut BoundsBudget,
    basis: CircleFocusBasis,
    check: &dyn Fn() -> bool,
) -> Result<NativeRadialLayout, RadialLayoutError> {
    cancel(check)?;
    if paths.len() > 4096 {
        return Err(RadialLayoutError::Limit("input paths"));
    }
    if options.coordinate_tolerance.raw() < 256 {
        return Err(RadialLayoutError::Invalid(
            "bounds tolerance below 256 Q32 units",
        ));
    }
    let (bounds, work) = bounds::collect(paths, options.coordinate_tolerance, budget, check)?;
    layout(fill, bounds, work, basis, check)
}
pub fn layout_background(
    fill: &EffectiveGradientFill,
    size: Size,
    check: &dyn Fn() -> bool,
) -> Result<NativeRadialLayout, RadialLayoutError> {
    layout_background_with_basis(fill, size, CircleFocusBasis::CircumscribedSquare, check)
}
pub fn layout_background_with_basis(
    fill: &EffectiveGradientFill,
    size: Size,
    basis: CircleFocusBasis,
    check: &dyn Fn() -> bool,
) -> Result<NativeRadialLayout, RadialLayoutError> {
    cancel(check)?;
    if size.width.get() <= 0 || size.height.get() <= 0 {
        return Err(RadialLayoutError::Invalid("background extent"));
    }
    layout(
        fill,
        [
            I::integer(0),
            I::integer(0),
            I::integer(size.width.get()),
            I::integer(size.height.get()),
        ],
        RadialLayoutWork {
            paths: 0,
            commands: 0,
            bounds_steps: 0,
            arc_segments: 0,
        },
        basis,
        check,
    )
}
fn layout(
    fill: &EffectiveGradientFill,
    bounds: [I; 4],
    work: RadialLayoutWork,
    basis: CircleFocusBasis,
    check: &dyn Fn() -> bool,
) -> Result<NativeRadialLayout, RadialLayoutError> {
    cancel(check)?;
    let EffectiveGradientShade::Path {
        path, fill_to_rect, ..
    } = &fill.shade
    else {
        return Err(RadialLayoutError::Invalid("circle gradient required"));
    };
    if path.value != NativePathShade::Circle {
        return Err(RadialLayoutError::Invalid("circle gradient required"));
    }
    let r = tile(&bounds, &fill.tile_rect)?;
    let w = r[2].sub(&r[0]);
    let h = r[3].sub(&r[1]);
    let radius = w
        .mul(&w)
        .add(&h.mul(&h))
        .sqrt_nonnegative()
        .map_err(numeric)?
        .divide(2);
    let center = [r[0].add(&r[2]).divide(2), r[1].add(&r[3]).divide(2)];
    let origin = [center[0].sub(&radius), center[1].sub(&radius)];
    let diameter = radius.mul(&I::integer(2));
    let p = percentages(fill_to_rect)?;
    let mut inner_center = Vec::new();
    let mut inner_radii = Vec::new();
    let mut focus = Vec::new();
    let mut scales = Vec::new();
    for i in 0..2 {
        cancel(check)?;
        let (ln, ld) = &p[i];
        let (rn, rd) = &p[i + 2];
        let denominator = ld * rd;
        let sum = ln * rd + rn * ld;
        if sum > denominator {
            return Err(RadialLayoutError::Invalid(
                "inverted radial focus rectangle",
            ));
        }
        // Classify exact source ratios, not widened interval endpoints. For an
        // unchanged extent (sum=0), preserve the documented origin fallback.
        let scale = ratio(&denominator - &sum, denominator);
        let left = ratio(ln.clone(), ld.clone());
        let center_factor = left.add(&scale.divide(2));
        inner_radii.push(radius.mul(&scale));
        let factor = if sum == BigInt::from(0) {
            left.clone()
        } else {
            ratio(ln * rd, sum.clone())
        };
        match basis {
            CircleFocusBasis::CircumscribedSquare => {
                inner_center.push(origin[i].add(&diameter.mul(&center_factor)));
                focus.push(origin[i].add(&diameter.mul(&factor)));
            }
            CircleFocusBasis::AnchorRectangle => {
                let extent = if i == 0 { &w } else { &h };
                focus.push(r[i].add(&extent.mul(&factor)));
                // C' = S*C + (1-S)*P. Avoid multiplying a very distant P by
                // tiny (1-S), which would amplify interval dependency. Exact
                // sum=0 uses finite P, so the center axis remains unchanged.
                inner_center.push(if sum == BigInt::from(0) {
                    center[i].clone()
                } else {
                    center[i]
                        .mul(&scale)
                        .add(&r[i].mul(&I::integer(1).sub(&scale)))
                        .add(&extent.mul(&left))
                });
            }
        }
        scales.push(scale);
    }
    cancel(check)?;
    Ok(NativeRadialLayout {
        profile: match basis {
            CircleFocusBasis::CircumscribedSquare => PROFILE,
            CircleFocusBasis::AnchorRectangle => ANCHOR_FOCUS_PROFILE,
        }
        .into(),
        path_bounds: estimate(bounds)?,
        tile_rectangle: estimate(r)?,
        outer_center: estimate(center)?,
        outer_radius: estimate([radius])?,
        inner_center: estimate(inner_center.try_into().expect("two axes"))?,
        inner_radii: estimate(inner_radii.try_into().expect("two axes"))?,
        focus_point: estimate(focus.try_into().expect("two axes"))?,
        focus_scale: estimate(scales.try_into().expect("two axes"))?,
        rotate_with_shape: fill.rotate_with_shape.value,
        work,
    })
}

#[cfg(test)]
mod tests;
