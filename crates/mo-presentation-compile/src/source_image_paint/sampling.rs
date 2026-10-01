//! Rebind logical source pixels to a smaller sample grid without changing crop,
//! density, tile phase, placement or clip geometry. Carry outward Q96 errors.
use super::*;

pub(crate) fn compile_sampled(
    plan: &ImageSourceLayoutPlan,
    image: &mo_image::DecodedImageInfo,
    sampling: ImageSampling,
    check: &dyn Fn() -> bool,
) -> Result<NativeImagePaint, ImagePaintError> {
    let mut paint = compile(plan, sampling, check)?;
    let source = image.source_size();
    if (source.width, source.height) == (image.width, image.height) {
        return Ok(paint);
    }
    let b = &mut paint.brush;
    let mut error = b
        .uncertainty
        .take()
        .map(|e| *e)
        .unwrap_or(ImageBrushUncertainty {
            origin: ZERO,
            x_step: ZERO,
            y_step: ZERO,
            source_domain: [Fixed::ZERO; 4],
        });
    let scale = [
        I::ratio(source.width.into(), image.width.into()),
        I::ratio(source.height.into(), image.height.into()),
    ];
    let step = |p: Point, e: Point, ratio: &I| -> Result<_, ImagePaintError> {
        point(&[enclose(p.x, e.x)?.mul(ratio), enclose(p.y, e.y)?.mul(ratio)])
    };
    (b.x_step, error.x_step) = step(b.x_step, error.x_step, &scale[0])?;
    (b.y_step, error.y_step) = step(b.y_step, error.y_step, &scale[1])?;
    if let Some(domain) = &mut b.source_domain {
        for (i, edge) in [
            &mut domain.left,
            &mut domain.top,
            &mut domain.right,
            &mut domain.bottom,
        ]
        .into_iter()
        .enumerate()
        {
            let value = enclose(*edge, error.source_domain[i])?
                .div_positive(&scale[i % 2])
                .map_err(|_| ImagePaintError::Precision)?;
            (*edge, error.source_domain[i]) = value.q32().map_err(|_| ImagePaintError::Range)?;
        }
    }
    b.uncertainty = Some(Box::new(error));
    cancel(check)?;
    Ok(paint)
}
