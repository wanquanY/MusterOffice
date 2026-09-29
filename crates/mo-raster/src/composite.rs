//! Immutable capture identity, source lifetime and shared pixel admission.
use crate::{
    BlendMode, Brush, CompositeWork, PathRasterRequest, RasterError, SnapshotScope, cancel,
};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Capture {
    pub after_draws: u32,
    /// Root is zero; group identities are one-based in the binary frame.
    pub scope: u32,
}
pub(crate) struct Composite {
    pub captures: Vec<Capture>,
    pub scoped: bool,
    pub work: Option<CompositeWork>,
}
pub(crate) fn capture(
    q: &PathRasterRequest,
    scopes: &[u32],
    after_draws: u32,
    scope: SnapshotScope,
) -> Result<Capture, RasterError> {
    let scope = match scope {
        SnapshotScope::Current => scopes.get(after_draws as usize).copied().unwrap_or(0),
        SnapshotScope::Output => 0,
        SnapshotScope::Group { index } => {
            let g = q
                .opacity_groups
                .get(index as usize)
                .ok_or(RasterError::Invalid("snapshot group reference"))?;
            if after_draws < g.first_draw || after_draws >= g.end_draw {
                return Err(RasterError::Invalid(
                    "snapshot group is not active at capture",
                ));
            }
            index + 1
        }
    };
    Ok(Capture { after_draws, scope })
}
pub(crate) fn prepare(
    q: &PathRasterRequest,
    groups: &crate::opacity::Groups,
    check: &dyn Fn() -> bool,
) -> Result<Composite, RasterError> {
    let mut captures = BTreeSet::new();
    let mut snapshot_draws = 0;
    let mut source_draws = 0;
    let mut scoped = false;
    for (i, draw) in q.draws.iter().enumerate() {
        cancel(check)?;
        source_draws += u32::from(draw.blend == BlendMode::Source);
        if let Brush::Snapshot { after_draws, scope } = draw.brush {
            if after_draws as usize > i {
                return Err(RasterError::Invalid("snapshot must precede its draw"));
            }
            let c = capture(q, &groups.scopes, after_draws, scope)?;
            if scope == SnapshotScope::Current
                && c.scope != groups.scopes.get(i).copied().unwrap_or(0)
            {
                return Err(RasterError::Invalid("snapshot crosses opacity group scope"));
            }
            scoped |= scope != SnapshotScope::Current && groups.work.is_some();
            captures.insert(c);
            // Stop before accepting additional capture storage.
            if captures.len() > 64 {
                return Err(RasterError::Limit("snapshots"));
            }
            snapshot_draws += 1;
        }
    }
    let bytes =
        u64::from(q.viewport.width) * u64::from(q.viewport.height) * 4 * captures.len() as u64;
    if bytes > crate::MAX_PIXEL_BYTES as u64 {
        return Err(RasterError::Limit("snapshot pixel bytes"));
    }
    if bytes
        + groups
            .work
            .as_ref()
            .map_or(0, |w| u64::from(w.peak_pixel_bytes))
        > crate::MAX_PIXEL_BYTES as u64
    {
        return Err(RasterError::Limit("opacity and snapshot pixel bytes"));
    }
    let work = (snapshot_draws != 0 || source_draws != 0).then_some(CompositeWork {
        captures: captures.len() as u32,
        captured_bytes: bytes as u32,
        snapshot_draws,
        source_draws,
    });
    Ok(Composite {
        captures: captures.into_iter().collect(),
        scoped,
        work,
    })
}
