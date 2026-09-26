//! Pure prefix-reference validation and bounded snapshot admission.
use crate::{BlendMode, Brush, CompositeWork, PathRasterRequest, RasterError, cancel};
use std::collections::BTreeSet;
pub(crate) struct Composite {
    pub prefixes: Vec<u32>,
    pub work: Option<CompositeWork>,
}
pub(crate) fn prepare(
    q: &PathRasterRequest,
    check: &dyn Fn() -> bool,
) -> Result<Composite, RasterError> {
    let mut prefixes = BTreeSet::new();
    let mut snapshot_draws = 0;
    let mut source_draws = 0;
    for (i, draw) in q.draws.iter().enumerate() {
        cancel(check)?;
        source_draws += u32::from(draw.blend == BlendMode::Source);
        if let Brush::Snapshot { after_draws } = draw.brush {
            if after_draws as usize > i {
                return Err(RasterError::Invalid("snapshot must precede its draw"));
            }
            prefixes.insert(after_draws);
            snapshot_draws += 1;
        }
    }
    if prefixes.len() > 64 {
        return Err(RasterError::Limit("snapshots"));
    }
    let bytes =
        u64::from(q.viewport.width) * u64::from(q.viewport.height) * 4 * prefixes.len() as u64;
    if bytes > crate::MAX_PIXEL_BYTES as u64 {
        return Err(RasterError::Limit("snapshot pixel bytes"));
    }
    let work = (snapshot_draws != 0 || source_draws != 0).then_some(CompositeWork {
        captures: prefixes.len() as u32,
        captured_bytes: bytes as u32,
        snapshot_draws,
        source_draws,
    });
    Ok(Composite {
        prefixes: prefixes.into_iter().collect(),
        work,
    })
}
