//! Isolated group grammar and resource admission, before backend allocation.
use crate::{OpacityGroupWork, PathRasterRequest, RasterError, cancel};

pub(crate) struct Groups {
    /// 0 is the output; positive values are one-based group identities.
    pub scopes: Vec<u32>,
    pub work: Option<OpacityGroupWork>,
}
pub(crate) fn prepare(
    q: &PathRasterRequest,
    check: &dyn Fn() -> bool,
) -> Result<Groups, RasterError> {
    if q.opacity_groups.is_empty() {
        return Ok(Groups {
            scopes: vec![],
            work: None,
        });
    }
    if q.opacity_groups.len() > 4096 {
        return Err(RasterError::Limit("opacity groups"));
    }
    let mut active = Vec::<usize>::new();
    let mut scopes = Vec::with_capacity(q.draws.len());
    let mut next = 0;
    let mut depth = 0;
    for i in 0..q.draws.len() {
        cancel(check)?;
        while active
            .last()
            .is_some_and(|&g| q.opacity_groups[g].end_draw as usize == i)
        {
            active.pop();
        }
        while let Some(g) = q.opacity_groups.get(next) {
            if g.first_draw as usize > i {
                break;
            }
            if g.first_draw as usize != i
                || g.end_draw <= g.first_draw
                || g.end_draw as usize > q.draws.len()
                || active
                    .last()
                    .is_some_and(|&p| g.end_draw > q.opacity_groups[p].end_draw)
            {
                return Err(RasterError::Invalid("opacity group interval or preorder"));
            }
            active.push(next);
            next += 1;
            depth = depth.max(active.len());
            if depth > 64 {
                return Err(RasterError::Limit("opacity group depth"));
            }
        }
        scopes.push(active.last().map_or(0, |&g| g as u32 + 1));
    }
    if next != q.opacity_groups.len() {
        return Err(RasterError::Invalid("opacity group interval or preorder"));
    }
    let pixels = u64::from(q.viewport.width) * u64::from(q.viewport.height);
    let peak = pixels * 4 * depth as u64;
    if peak > crate::MAX_PIXEL_BYTES as u64 {
        return Err(RasterError::Limit("opacity and snapshot pixel bytes"));
    }
    let work = pixels * 2 * q.opacity_groups.len() as u64;
    if work > 268_435_456 {
        return Err(RasterError::Limit("opacity group pixel work"));
    }
    Ok(Groups {
        scopes,
        work: Some(OpacityGroupWork {
            groups: q.opacity_groups.len() as u32,
            maximum_depth: depth as u32,
            peak_pixel_bytes: peak as u32,
            pixel_work: work as u32,
        }),
    })
}
