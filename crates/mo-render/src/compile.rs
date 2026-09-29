use crate::{SceneRasterRequest, SceneWork, cancel};
use mo_geometry::{Affine, Fixed, GeometryError, PathCommand, Point, PointEstimate};
use mo_raster::{CompiledRaster, FillPath, PathDraw, PathRasterRequest, RasterError};
use std::collections::BTreeMap;
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
pub struct CompiledScene {
    pub(crate) raster: CompiledRaster,
    pub(crate) work: SceneWork,
}
impl CompiledScene {
    pub fn raster(&self) -> &CompiledRaster {
        &self.raster
    }
    pub fn work(&self) -> &SceneWork {
        &self.work
    }
}
fn geometry(e: GeometryError) -> RasterError {
    match e {
        GeometryError::Cancelled => RasterError::Cancelled,
        GeometryError::Invalid(s) => RasterError::Invalid(s),
        GeometryError::Limit(s) => RasterError::Limit(s),
        GeometryError::Numeric => RasterError::Range,
    }
}
fn points(c: &PathCommand) -> ([Point; 3], usize) {
    match *c {
        PathCommand::Move { to } | PathCommand::Line { to } => ([to, ZERO, ZERO], 1),
        PathCommand::Quadratic { control, to } => ([control, to, ZERO], 2),
        PathCommand::Cubic {
            control1,
            control2,
            to,
        } => ([control1, control2, to], 3),
        PathCommand::Close => ([ZERO; 3], 0),
    }
}
fn with_points(c: &PathCommand, p: [Point; 3]) -> PathCommand {
    match c {
        PathCommand::Move { .. } => PathCommand::Move { to: p[0] },
        PathCommand::Line { .. } => PathCommand::Line { to: p[0] },
        PathCommand::Quadratic { .. } => PathCommand::Quadratic {
            control: p[0],
            to: p[1],
        },
        PathCommand::Cubic { .. } => PathCommand::Cubic {
            control1: p[0],
            control2: p[1],
            to: p[2],
        },
        PathCommand::Close => PathCommand::Close,
    }
}
fn anchor(path: &FillPath) -> Point {
    match path.commands.first() {
        Some(PathCommand::Move { to }) => *to,
        _ => ZERO,
    }
}
pub fn compile(
    request: &SceneRasterRequest,
    check: &dyn Fn() -> bool,
) -> Result<CompiledScene, RasterError> {
    let (raster, work) = compile_with(request, check, &|paths, check| {
        let raster = mo_raster::compile(paths, check)?;
        let bound = raster.work().coordinate_error_bound;
        Ok((raster, bound))
    })?;
    Ok(CompiledScene { raster, work })
}
/// Share geometry lowering, resource interning and precision retries between
/// plain and image-bearing scenes. Only the final raster resource binding varies.
pub(crate) fn compile_with<T>(
    request: &SceneRasterRequest,
    check: &dyn Fn() -> bool,
    finish: &impl Fn(&PathRasterRequest, &dyn Fn() -> bool) -> Result<(T, Fixed), RasterError>,
) -> Result<(T, SceneWork), RasterError> {
    match attempt(request, check, false, finish) {
        Err(RasterError::Range | RasterError::Precision) => attempt(request, check, true, finish),
        result => result,
    }
}
fn evaluated(
    point: Point,
    chain: &[Affine],
    origin: Point,
    check: &dyn Fn() -> bool,
) -> Result<PointEstimate, RasterError> {
    let mut p = PointEstimate { point, error: ZERO };
    for (i, transform) in chain.iter().enumerate() {
        cancel(check)?;
        p = transform
            .map_in_view(p, if i + 1 == chain.len() { origin } else { ZERO })
            .map_err(geometry)?;
    }
    if chain.is_empty() {
        p = Affine::IDENTITY.map_in_view(p, origin).map_err(geometry)?;
    }
    Ok(p)
}
fn attempt<T>(
    request: &SceneRasterRequest,
    check: &dyn Fn() -> bool,
    original_chain: bool,
    finish: &impl Fn(&PathRasterRequest, &dyn Fn() -> bool) -> Result<(T, Fixed), RasterError>,
) -> Result<(T, SceneWork), RasterError> {
    let (lowered, mut work) = lower(request, check, original_chain)?;
    let (raster, device_error) = finish(&lowered, check)?;
    let v = &request.viewport;
    work.combined_coordinate_error_bound = work
        .transform_error_bound
        .ratio_up(v.scale.numerator, v.scale.denominator)
        .map_err(geometry)?
        .checked_add(device_error)
        .map_err(geometry)?;
    if work.combined_coordinate_error_bound > v.coordinate_tolerance {
        return Err(RasterError::Precision);
    }
    cancel(check)?;
    Ok((raster, work))
}
// Keep the substantial geometry computation non-generic: adding a resource
// binding must not force a second monomorphization of the entire algorithm.
fn lower(
    request: &SceneRasterRequest,
    check: &dyn Fn() -> bool,
    original_chain: bool,
) -> Result<(PathRasterRequest, SceneWork), RasterError> {
    cancel(check)?;
    let scene = &request.scene;
    // Validate device policy before potentially expensive geometry work.
    mo_raster::compile(
        &PathRasterRequest {
            opacity_groups: vec![],
            clips: vec![],
            viewport: request.viewport.clone(),
            paths: vec![],
            draws: vec![],
        },
        check,
    )?;
    if scene.paths.len() > 4096
        || scene.opacity_groups.len() > 4096
        || scene.transforms.len() > 8192
        || scene.instances.len() > mo_raster::MAX_DRAWS
    {
        return Err(RasterError::Limit("scene resources or instances"));
    }
    let mut source_commands = 0usize;
    let mut point_counts = Vec::with_capacity(scene.paths.len());
    for path in &scene.paths {
        cancel(check)?;
        source_commands += path.commands.len();
        if source_commands > 262144 {
            return Err(RasterError::Limit("scene source commands"));
        }
        let mut contour = false;
        let mut count = 0u32;
        for (i, c) in path.commands.iter().enumerate() {
            cancel(check)?;
            match c {
                PathCommand::Move { .. } => contour = true,
                _ if !contour => {
                    return Err(RasterError::Invalid(if i == 0 {
                        "path must start with move"
                    } else {
                        "path contour order"
                    }));
                }
                PathCommand::Close => contour = false,
                _ => {}
            }
            count += points(c).1 as u32;
        }
        point_counts.push(count);
    }
    if scene.clips.len() > 8192 {
        return Err(RasterError::Limit("scene clip nodes"));
    }
    let mut clip_needed = vec![false; scene.clips.len()];
    let mut clip_depths = Vec::with_capacity(scene.clips.len());
    for (i, clip) in scene.clips.iter().enumerate() {
        cancel(check)?;
        if clip.path as usize >= scene.paths.len()
            || clip
                .transform
                .is_some_and(|t| t as usize >= scene.transforms.len())
        {
            return Err(RasterError::Invalid("scene clip resource reference"));
        }
        let depth = match clip.parent {
            Some(p) if (p as usize) < i => clip_depths[p as usize] + 1,
            Some(_) => return Err(RasterError::Invalid("clip parent order")),
            None => 1u32,
        };
        if depth > 64 {
            return Err(RasterError::Limit("clip depth"));
        }
        clip_depths.push(depth);
    }
    for instance in &scene.instances {
        cancel(check)?;
        if let Some(c) = instance.clip {
            *clip_needed
                .get_mut(c as usize)
                .ok_or(RasterError::Invalid("scene clip reference"))? = true;
        }
    }
    for (i, clip) in scene.clips.iter().enumerate().rev() {
        cancel(check)?;
        if clip_needed[i]
            && let Some(parent) = clip.parent
        {
            clip_needed[parent as usize] = true;
        }
    }
    let used_clips: Vec<usize> = clip_needed
        .iter()
        .enumerate()
        .filter_map(|(i, &needed)| needed.then_some(i))
        .collect();
    // Clip and draw geometry use the same interning, precision and retry path.
    let uses: Vec<(u32, Option<u32>)> = used_clips
        .iter()
        .map(|&i| (scene.clips[i].path, scene.clips[i].transform))
        .chain(scene.instances.iter().map(|i| (i.path, i.transform)))
        .collect();
    // Validate all references, but compute only ancestors of nonempty uses.
    // Unused resources must not alter numerical strategy or expand path copies.
    let mut needed = vec![false; scene.transforms.len()];
    let mut paint_budget = mo_raster::PaintBudget::default();
    for instance in &scene.instances {
        cancel(check)?;
        paint_budget.include_brush(&instance.brush)?;
        let path = scene
            .paths
            .get(instance.path as usize)
            .ok_or(RasterError::Invalid("scene path reference"))?;
        if let Some(t) = instance.transform {
            let slot = needed
                .get_mut(t as usize)
                .ok_or(RasterError::Invalid("scene transform reference"))?;
            *slot |= !path.commands.is_empty();
        }
    }
    for &i in &used_clips {
        let clip = &scene.clips[i];
        if let Some(t) = clip.transform {
            needed[t as usize] |= !scene.paths[clip.path as usize].commands.is_empty();
        }
    }
    for (i, node) in scene.transforms.iter().enumerate().rev() {
        cancel(check)?;
        if let Some(parent) = node.parent {
            if parent as usize >= i {
                return Err(RasterError::Invalid("transform parent order"));
            }
            if needed[i] {
                needed[parent as usize] = true;
            }
        }
    }
    let mut world: Vec<Affine> = Vec::with_capacity(scene.transforms.len());
    let mut depths: Vec<u32> = Vec::with_capacity(scene.transforms.len());
    for (i, node) in scene.transforms.iter().enumerate() {
        cancel(check)?;
        let (affine, depth) = if let Some(parent) = node.parent {
            if parent as usize >= i {
                return Err(RasterError::Invalid("transform parent order"));
            }
            (
                if original_chain || !needed[i] {
                    Affine::IDENTITY
                } else {
                    world[parent as usize]
                        .compose(node.affine)
                        .map_err(geometry)?
                },
                depths[parent as usize] + 1,
            )
        } else {
            (
                if original_chain || !needed[i] {
                    Affine::IDENTITY
                } else {
                    node.affine
                        .rebase(request.viewport.origin)
                        .map_err(geometry)?
                },
                1,
            )
        };
        if depth > 64 {
            return Err(RasterError::Limit("transform depth"));
        }
        world.push(affine);
        depths.push(depth);
    }
    let mut work = SceneWork {
        clips: (!scene.clips.is_empty()).then_some(crate::SceneClipWork {
            source_nodes: scene.clips.len() as u32,
            compiled_nodes: used_clips.len() as u32,
        }),
        lowering_attempts: if original_chain { 2 } else { 1 },
        source_paths: scene.paths.len() as u32,
        source_commands: source_commands as u32,
        transforms: world.len() as u32,
        maximum_depth: depths.iter().copied().max().unwrap_or(0),
        evaluated_points: 0,
        point_transform_work: 0,
        compiled_paths: 0,
        compiled_commands: 0,
        transform_error_bound: Fixed::ZERO,
        combined_coordinate_error_bound: Fixed::ZERO,
    };
    let mut drawn_commands = 0usize;
    for (use_index, &(path_id, transform)) in uses.iter().enumerate() {
        cancel(check)?;
        let path = scene
            .paths
            .get(path_id as usize)
            .ok_or(RasterError::Invalid("scene path reference"))?;
        let depth = match transform {
            None => 1,
            Some(t) => *depths
                .get(t as usize)
                .ok_or(RasterError::Invalid("scene transform reference"))?,
        };
        if use_index >= used_clips.len() {
            drawn_commands += path.commands.len();
        }
        if drawn_commands > 1048576 {
            return Err(RasterError::Limit("drawn commands"));
        }
        let points = point_counts[path_id as usize];
        work.evaluated_points += points;
        work.point_transform_work = work
            .point_transform_work
            .checked_add(points * depth)
            .ok_or(RasterError::Limit("point transform work"))?;
        if work.point_transform_work > 4194304 {
            return Err(RasterError::Limit("point transform work"));
        }
    }
    let mut lowered = PathRasterRequest {
        opacity_groups: scene.opacity_groups.clone(),
        clips: vec![],
        viewport: request.viewport.clone(),
        paths: vec![],
        draws: Vec::with_capacity(scene.instances.len()),
    };
    lowered.viewport.origin = ZERO;
    // Cache only the linear part: glyph placement changes translation, not its
    // outline resource. Quantized world matrices are optimization candidates;
    // every used control point is certified against the original node chain.
    let mut cache = BTreeMap::new();
    let mut clip_indices = vec![None; scene.clips.len()];
    for (use_index, &(path_id, transform)) in uses.iter().enumerate() {
        cancel(check)?;
        let path = &scene.paths[path_id as usize];
        let a = anchor(path);
        let matrix = if original_chain {
            Affine::IDENTITY
        } else {
            match transform {
                Some(t) => world[t as usize],
                None => Affine::IDENTITY
                    .rebase(request.viewport.origin)
                    .map_err(geometry)?,
            }
        };
        let mut chain = Vec::with_capacity(64);
        let mut node = transform;
        while let Some(i) = node {
            chain.push(scene.transforms[i as usize].affine);
            node = scene.transforms[i as usize].parent;
        }
        let key = (
            path_id,
            if original_chain {
                Affine::IDENTITY.linear
            } else {
                matrix.linear
            },
            if original_chain { transform } else { None },
        );
        let index = if let Some(index) = cache.get(&key) {
            *index
        } else {
            if lowered.paths.len() == 4096
                || work.compiled_commands as usize + path.commands.len() > 262144
            {
                return Err(RasterError::Limit("compiled path resources"));
            }
            let linear = Affine {
                linear: matrix.linear,
                translation: ZERO,
            };
            let mut commands = Vec::with_capacity(path.commands.len());
            for c in &path.commands {
                cancel(check)?;
                let (mut p, n) = points(c);
                for point in &mut p[..n] {
                    if original_chain {
                        let exact = evaluated(*point, &chain, request.viewport.origin, check)?;
                        work.transform_error_bound = work
                            .transform_error_bound
                            .max(exact.error.x)
                            .max(exact.error.y);
                        *point = exact.point;
                    } else {
                        *point = linear.map_vector_from(*point, a).map_err(geometry)?;
                    }
                }
                commands.push(with_points(c, p));
            }
            let index = lowered.paths.len() as u32;
            work.compiled_commands += commands.len() as u32;
            lowered.paths.push(FillPath {
                fill_rule: path.fill_rule,
                commands,
            });
            cache.insert(key, index);
            index
        };
        let origin = if original_chain || path.commands.is_empty() {
            ZERO
        } else {
            matrix.map(a).map_err(geometry)?.point
        };
        for (source, compiled) in path
            .commands
            .iter()
            .zip(&lowered.paths[index as usize].commands)
            .take(if original_chain {
                0
            } else {
                path.commands.len()
            })
        {
            cancel(check)?;
            let (source, n) = points(source);
            let (compiled, _) = points(compiled);
            for k in 0..n {
                let exact = evaluated(source[k], &chain, request.viewport.origin, check)?;
                for (p, o, r, e) in [
                    (compiled[k].x, origin.x, exact.point.x, exact.error.x),
                    (compiled[k].y, origin.y, exact.point.y, exact.error.y),
                ] {
                    let bound = Fixed::sum_deviation(p, o, r)
                        .map_err(geometry)?
                        .checked_add(e)
                        .map_err(geometry)?;
                    work.transform_error_bound = work.transform_error_bound.max(bound);
                }
            }
        }
        if use_index < used_clips.len() {
            let source_index = used_clips[use_index];
            let clip = &scene.clips[source_index];
            clip_indices[source_index] = Some(lowered.clips.len() as u32);
            lowered.clips.push(mo_raster::PathClip {
                parent: clip.parent.and_then(|i| clip_indices[i as usize]),
                path: index,
                origin,
            });
        } else {
            let instance = &scene.instances[use_index - used_clips.len()];
            lowered.draws.push(PathDraw {
                blend: instance.blend,
                clip: instance.clip.and_then(|i| clip_indices[i as usize]),
                path: index,
                origin,
                brush: instance.brush.rebased(request.viewport.origin)?,
                stroke: instance.stroke,
            });
        }
    }
    work.compiled_paths = lowered.paths.len() as u32;
    Ok((lowered, work))
}
