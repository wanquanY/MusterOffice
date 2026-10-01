//! Viewport-aware decode demand, computed before allocating any pixel buffers.
//! Every use of a resource contributes; one tiled use retains its exact grid.
use super::resources::Pending;
use super::*;
use crate::{source_image_layout, source_image_paint};
use mo_geometry::{Fixed, Point};
use mo_image::DecodeSize;
use mo_raster::ImageSampling;

/// A retained plan has no decoder after preparation. Only owners whose scale
/// and orientation are fixed over the admitted timeline may use viewport grids.
#[derive(Clone, Copy)]
pub(crate) enum DecodePolicy<'a> {
    Viewport,
    Exact,
    Retained(&'a std::collections::BTreeSet<(String, u32)>),
}

pub(super) fn demands(
    index: &SourceIndex,
    page: &source_page::PreparedPage<'_>,
    pending: &[Pending<'_>],
    count: usize,
    sampling: ImageSampling,
    policy: DecodePolicy<'_>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Option<DecodeSize>>, SourcePageError> {
    // Nearest explicitly requests source texels, so resampling is inappropriate.
    if matches!(sampling, ImageSampling::Nearest) || matches!(policy, DecodePolicy::Exact) {
        return Ok(vec![None; count]);
    }
    let mut demands = vec![
        Some(DecodeSize {
            width: 1,
            height: 1
        });
        count
    ];
    for item in pending {
        cancel(check)?;
        let slot = &mut demands[item.resource as usize];
        if let DecodePolicy::Retained(dynamic) = policy
            && item
                .owner
                .location
                .object
                .is_some_and(|id| dynamic.contains(&(item.owner.location.part.clone(), id)))
        {
            *slot = None;
        }
        if slot.is_none() {
            continue;
        }
        let placement = item.owner.placement.clone();
        let size = placement
            .as_ref()
            .map(|p| p.source_size)
            .or(index.page_size)
            .ok_or(SourcePageError::Invalid("page image source size"))?;
        let region = item
            .owner
            .region
            .as_ref()
            .map(|r| (r.bounds, r.coordinate_error_bound));
        let Some(layout) =
            source_image_layout::unit_stretch_layout(&item.source.image, size, region, check)
                .map_err(|e| SourcePageError::from(e).at(&item.owner.location))?
        else {
            *slot = None;
            continue;
        };
        let paint = source_image_paint::compile(
            &source_image_layout::ImageSourceLayoutPlan {
                target: item.target.clone(),
                resource: item.resource,
                placement,
                layout,
            },
            sampling,
            check,
        )
        .map_err(|e| SourcePageError::from(e).at(&item.owner.location))?;
        let brush = paint.brush;
        let zero = Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        };
        let error = brush.uncertainty.as_deref();
        let axis = |step: Point, error: Point| -> u32 {
            // L1 bounds the rotated/skewed footprint without floating rounding.
            // Two samples/device pixel preserve detail during linear filtering.
            let raw = step.x.raw().abs() + step.y.raw().abs() + error.x.raw() + error.y.raw();
            let n = raw * i128::from(page.viewport.scale.numerator) * 2;
            let d = (1i128 << 32) * i128::from(page.viewport.scale.denominator);
            ((n + d - 1) / d).clamp(1, 8192) as u32
        };
        let demand = slot.as_mut().expect("exact-grid cases were excluded");
        demand.width = demand
            .width
            .max(axis(brush.x_step, error.map_or(zero, |e| e.x_step)));
        demand.height = demand
            .height
            .max(axis(brush.y_step, error.map_or(zero, |e| e.y_step)));
    }
    Ok(demands)
}
