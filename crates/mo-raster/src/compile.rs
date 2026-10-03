use crate::{
    Brush, FillRule, ImageWork, PathRasterRequest, PreparedImages, RasterError, RasterWork, cancel,
    image::ImageBrushes,
    number::{Scale, add_sub},
};
use mo_geometry::{Fixed, PathCommand, Point};
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
pub struct CompiledRaster {
    frame: Vec<u32>,
    pub(crate) work: RasterWork,
    pixel_bytes: usize,
}
impl CompiledRaster {
    pub fn work(&self) -> &RasterWork {
        &self.work
    }
    pub fn width(&self) -> u32 {
        self.frame[2]
    }
    pub fn height(&self) -> u32 {
        self.frame[3]
    }
    pub fn frame(&self) -> &[u32] {
        &self.frame
    }
    pub fn pixel_bytes(&self) -> usize {
        self.pixel_bytes
    }
}
fn coordinates(c: &PathCommand) -> (u32, [Point; 3], usize) {
    match *c {
        PathCommand::Move { to } => (1, [to, ZERO, ZERO], 1),
        PathCommand::Line { to } => (2, [to, ZERO, ZERO], 1),
        PathCommand::Quadratic { control, to } => (3, [control, to, ZERO], 2),
        PathCommand::Cubic {
            control1,
            control2,
            to,
        } => (4, [control1, control2, to], 3),
        PathCommand::Close => (5, [ZERO; 3], 0),
    }
}
fn axes(p: Point) -> [i128; 2] {
    [p.x.raw(), p.y.raw()]
}
pub fn compile(
    request: &PathRasterRequest,
    check: &dyn Fn() -> bool,
) -> Result<CompiledRaster, RasterError> {
    Ok(compile_inner(request, None, check)?.0)
}
pub(crate) fn compile_inner(
    request: &PathRasterRequest,
    images: Option<&PreparedImages<'_>>,
    check: &dyn Fn() -> bool,
) -> Result<(CompiledRaster, ImageWork), RasterError> {
    cancel(check)?;
    let v = &request.viewport;
    v.validate()?;
    let scale = Scale::new(v.scale)?;
    if request.paths.len() > 4096 || request.draws.len() > crate::MAX_DRAWS {
        return Err(RasterError::Limit("paths or draws"));
    }
    let mut total = 0usize;
    for path in &request.paths {
        cancel(check)?;
        total = total
            .checked_add(path.commands.len())
            .filter(|v| *v <= 262144)
            .ok_or(RasterError::Limit("path commands"))?;
    }
    let mut work = 0usize;
    for draw in &request.draws {
        cancel(check)?;
        let path = request
            .paths
            .get(draw.path as usize)
            .ok_or(RasterError::Invalid("draw path reference"))?;
        work += path.commands.len();
        if work > 1048576 {
            return Err(RasterError::Limit("drawn commands"));
        }
    }
    let groups = crate::opacity::prepare(request, check)?;
    let composite = crate::composite::prepare(request, &groups, check)?;
    let clip_work = crate::clip::prepare(request, &groups.scopes, check)?;
    let mut strokes = crate::stroke::Strokes::default();
    let mut paints = Vec::with_capacity(request.draws.len());
    let mut gradients = crate::gradient::Gradients::default();
    let mut brushes = Vec::with_capacity(request.draws.len());
    let mut image_brushes = ImageBrushes::default();
    let mut image_draws = 0;
    let mut gradient_draws = 0;
    let mut stroke_draws = 0;
    for draw in &request.draws {
        cancel(check)?;
        let brush = if let Brush::Snapshot { after_draws, scope } = &draw.brush {
            let capture = crate::composite::capture(request, &groups.scopes, *after_draws, *scope)?;
            (
                0,
                0,
                0,
                composite
                    .captures
                    .binary_search(&capture)
                    .expect("admitted snapshot") as u32
                    + 1,
            )
        } else if let Brush::Image { image } = &draw.brush {
            let images = images.ok_or(RasterError::Invalid("image resources required"))?;
            image_draws += 1;
            (0, 0, image_brushes.insert(image, images, v)?, 0)
        } else {
            let (color, gradient) = gradients.insert(&draw.brush, v, check)?;
            gradient_draws += u32::from(gradient != 0);
            (color, gradient, 0, 0)
        };
        brushes.push(brush);
        paints.push(if let Some(style) = &draw.stroke {
            stroke_draws += 1;
            strokes.insert(style, v.scale, v.coordinate_tolerance)?
        } else {
            0
        });
    }
    let has_composite = groups.work.is_some()
        || composite.work.is_some()
        || gradients.has_planes
        || gradients.has_office_gamma;
    let has_clips = clip_work.is_some() || has_composite;
    let brush_words = if image_brushes.has_domains || has_clips {
        14
    } else {
        10
    };
    let extra = images.map_or(0, |i| {
        2 + i.len() * 4 + image_brushes.words.len() * brush_words
    });
    let count = 10
        + if groups.work.is_some() {
            1 + request.opacity_groups.len() * 3
        } else {
            0
        }
        + if has_composite {
            1 + composite.captures.len() * if composite.scoped { 2 } else { 1 }
                + request.draws.len()
        } else {
            0
        }
        + extra
        + if has_clips {
            1 + if images.is_none() { 2 } else { 0 } + request.clips.len() * 4 + request.draws.len()
        } else {
            0
        }
        + request.paths.len() * 2
        + total * 7
        + strokes.words.len() * 4
        + request.draws.len() * 6
        + gradients.words.iter().map(Vec::len).sum::<usize>();
    let mut frame = Vec::new();
    frame
        .try_reserve_exact(count)
        .map_err(|_| RasterError::Host("draw frame allocation"))?;
    frame.extend_from_slice(&[
        0x4d4f534b,
        if composite.scoped {
            14
        } else if groups.work.is_some() {
            13
        } else if gradients.elliptic.is_some() {
            12
        } else if gradients.has_rectangular {
            11
        } else if gradients.has_office_gamma {
            10
        } else if gradients.has_planes {
            9
        } else if has_composite {
            8
        } else if has_clips {
            7
        } else if image_brushes.has_domains {
            6
        } else if images.is_some() {
            5
        } else {
            4
        },
        v.width,
        v.height,
        u32::from_le_bytes(v.background),
        request.paths.len() as u32,
        request.draws.len() as u32,
        total as u32,
        strokes.words.len() as u32,
        gradients.words.len() as u32,
    ]);
    if let Some(images) = images {
        frame.extend_from_slice(&[images.len() as u32, image_brushes.words.len() as u32]);
    }
    if has_clips {
        if images.is_none() {
            frame.extend_from_slice(&[0, 0]);
        }
        frame.push(request.clips.len() as u32);
    }
    if has_composite {
        frame.push(composite.captures.len() as u32);
    }
    if groups.work.is_some() {
        frame.push(request.opacity_groups.len() as u32);
    }
    let mut origins = Vec::new();
    origins
        .try_reserve_exact(request.paths.len())
        .map_err(|_| RasterError::Host("path origin allocation"))?;
    for path in &request.paths {
        cancel(check)?;
        let anchor = match path.commands.first() {
            None => ZERO,
            Some(PathCommand::Move { to }) => *to,
            _ => return Err(RasterError::Invalid("path must start with move")),
        };
        frame.extend_from_slice(&[
            match path.fill_rule {
                FillRule::Nonzero => 0,
                FillRule::Evenodd => 1,
            },
            path.commands.len() as u32,
        ]);
        origins.push((axes(anchor), frame.len()));
        let mut contour = false;
        for c in &path.commands {
            cancel(check)?;
            let (op, points, n) = coordinates(c);
            if op != 1 && !contour {
                return Err(RasterError::Invalid("path contour order"));
            }
            if op == 1 {
                contour = true;
            }
            if op == 5 {
                contour = false;
            }
            let mut command = [0; 7];
            command[0] = op;
            for (i, p) in points[..n].iter().enumerate() {
                for (axis, value) in axes(*p).iter().enumerate() {
                    let local = value
                        .checked_sub(axes(anchor)[axis])
                        .ok_or(RasterError::Range)?;
                    command[1 + 2 * i + axis] = scale.value(local)?.to_bits();
                }
            }
            frame.extend_from_slice(&command);
        }
    }
    for words in &strokes.words {
        frame.extend_from_slice(words);
    }
    for words in &gradients.words {
        frame.extend_from_slice(words);
    }
    if let Some(images) = images {
        for descriptor in &images.descriptors {
            frame.extend_from_slice(descriptor);
        }
        for brush in &image_brushes.words {
            frame.extend_from_slice(&brush[..brush_words]);
        }
    }
    let mut error_bound = 0;
    for clip in &request.clips {
        cancel(check)?;
        let device = placement(
            request,
            clip.path,
            clip.origin,
            &origins,
            &frame,
            &scale,
            0.0,
            &mut error_bound,
            check,
        )?;
        frame.extend_from_slice(&[
            clip.parent.map_or(0, |i| i + 1),
            clip.path,
            device[0].to_bits(),
            device[1].to_bits(),
        ]);
    }
    for capture in &composite.captures {
        frame.push(capture.after_draws);
        if composite.scoped {
            frame.push(capture.scope);
        }
    }
    for group in &request.opacity_groups {
        frame.extend_from_slice(&[group.first_draw, group.end_draw, u32::from(group.opacity)]);
    }
    for ((draw, paint), (color, gradient, image, snapshot)) in
        request.draws.iter().zip(paints).zip(brushes)
    {
        cancel(check)?;
        let device = placement(
            request,
            draw.path,
            draw.origin,
            &origins,
            &frame,
            &scale,
            strokes.inflation(paint),
            &mut error_bound,
            check,
        )?;
        frame.extend_from_slice(&[
            draw.path,
            device[0].to_bits(),
            device[1].to_bits(),
            color,
            paint,
            if snapshot != 0 {
                gradients.words.len() as u32 + image_brushes.words.len() as u32 + snapshot
            } else if image == 0 {
                gradient
            } else {
                gradients.words.len() as u32 + image
            },
        ]);
        if has_clips {
            frame.push(draw.clip.map_or(0, |i| i + 1));
        }
        if has_composite {
            frame.push(u32::from(draw.blend == crate::BlendMode::Source));
        }
    }
    debug_assert_eq!(frame.len(), count);
    cancel(check)?;
    Ok((
        CompiledRaster {
            frame,
            pixel_bytes: v.width as usize * v.height as usize * 4,
            work: RasterWork {
                opacity_groups: groups.work,
                elliptic_gradients: gradients.elliptic,
                compositing: composite.work,
                clips: clip_work,
                paths: request.paths.len() as u32,
                commands: total as u32,
                draws: request.draws.len() as u32,
                drawn_commands: work as u32,
                coordinate_error_bound: Fixed::from_raw(error_bound),
                stroke_styles: strokes.words.len() as u32,
                stroke_draws,
                stroke_width_error_bound: strokes.width_error,
                miter_limit_error_bound: strokes.miter_error,
                gradients: gradients.words.len() as u32,
                gradient_stops: gradients.stored_stops,
                gradient_draws,
                gradient_coordinate_error_bound: gradients.coordinate_error,
                gradient_value_error_bound: gradients.value_error,
            },
        },
        ImageWork {
            resources: images.map_or(0, |i| i.len() as u32),
            resource_bytes: images.map_or(0, |i| i.bytes().len() as u32),
            brushes: image_brushes.words.len() as u32,
            draws: image_draws,
            coordinate_error_bound: image_brushes.coordinate_error,
        },
    ))
}

// Shared by clip paths and draw paths, including anchor rebasing and final
// float32 addition error. Clip geometry never borrows a draw's stroke inflation.
#[allow(clippy::too_many_arguments)]
fn placement(
    request: &PathRasterRequest,
    path_index: u32,
    origin: Point,
    origins: &[([i128; 2], usize)],
    frame: &[u32],
    scale: &Scale,
    inflation: f64,
    error_bound: &mut i128,
    check: &dyn Fn() -> bool,
) -> Result<[f32; 2], RasterError> {
    let path = &request.paths[path_index as usize];
    let (anchor, start) = origins[path_index as usize];
    let v = &request.viewport;
    let tolerance = v.coordinate_tolerance.raw();
    let mut offset = [0; 2];
    let mut device = [0.0f32; 2];
    if !path.commands.is_empty() {
        for k in 0..2 {
            offset[k] = add_sub(axes(origin)[k], anchor[k], axes(v.origin)[k])?;
            device[k] = scale.value(offset[k])?;
        }
    }
    for (j, c) in path.commands.iter().enumerate() {
        cancel(check)?;
        let (_, points, n) = coordinates(c);
        for (i, p) in points[..n].iter().enumerate() {
            for (k, value) in axes(*p).iter().enumerate() {
                let local = f32::from_bits(frame[start + 7 * j + 1 + 2 * i + k]);
                // Same pre-transform range condition as the C++ component.
                if (f64::from(local) + f64::from(device[k])).abs() + inflation > 32768.0 {
                    return Err(RasterError::Range);
                }
                let combined = add_sub(*value, offset[k], anchor[k])?;
                let bound = scale.error(combined, local + device[k])?;
                *error_bound = (*error_bound).max(bound);
                if bound > tolerance {
                    return Err(RasterError::Precision);
                }
            }
        }
    }
    Ok(device)
}
