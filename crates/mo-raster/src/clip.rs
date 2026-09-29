use crate::{ClipWork, PathRasterRequest, RasterError, cancel};

/// Bound traversal and actual raster work independently of draw command work.
pub(crate) fn prepare(
    request: &PathRasterRequest,
    scopes: &[u32],
    check: &dyn Fn() -> bool,
) -> Result<Option<ClipWork>, RasterError> {
    if request.clips.len() > 8192 {
        return Err(RasterError::Limit("clip nodes"));
    }
    let mut placement_commands = 0u32;
    let mut depths = Vec::with_capacity(request.clips.len());
    for (i, clip) in request.clips.iter().enumerate() {
        cancel(check)?;
        if clip.path as usize >= request.paths.len() {
            return Err(RasterError::Invalid("clip path reference"));
        }
        placement_commands += request.paths[clip.path as usize].commands.len() as u32;
        if placement_commands > 1048576 {
            return Err(RasterError::Limit("clip placement commands"));
        }
        let depth = match clip.parent {
            Some(parent) if (parent as usize) < i => depths[parent as usize] + 1,
            Some(_) => return Err(RasterError::Invalid("clip parent order")),
            None => 1u32,
        };
        if depth > 64 {
            return Err(RasterError::Limit("clip depth"));
        }
        depths.push(depth);
    }
    let mut work = ClipWork {
        placement_commands,
        nodes: request.clips.len() as u32,
        maximum_depth: depths.iter().copied().max().unwrap_or(0),
        applications: 0,
        applied_commands: 0,
    };
    let mut active = [0u32; 64];
    let mut active_len = 0;
    for (draw_index, draw) in request.draws.iter().enumerate() {
        cancel(check)?;
        if draw_index > 0 && scopes.get(draw_index) != scopes.get(draw_index - 1) {
            active_len = 0;
        }
        let mut chain = [0u32; 64];
        let mut len = 0;
        let mut next = draw.clip;
        while let Some(i) = next {
            let clip = request
                .clips
                .get(i as usize)
                .ok_or(RasterError::Invalid("draw clip reference"))?;
            chain[len] = i;
            len += 1;
            next = clip.parent;
        }
        chain[..len].reverse();
        let common = active[..active_len]
            .iter()
            .zip(&chain[..len])
            .take_while(|(a, b)| a == b)
            .count();
        for &i in &chain[common..len] {
            cancel(check)?;
            work.applications += 1;
            work.applied_commands += request.paths[request.clips[i as usize].path as usize]
                .commands
                .len() as u32;
            if work.applications > 262144 || work.applied_commands > 1048576 {
                return Err(RasterError::Limit("applied clip work"));
            }
        }
        active = chain;
        active_len = len;
    }
    Ok((!request.clips.is_empty()).then_some(work))
}
