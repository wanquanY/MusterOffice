//! DrawingML image fill layout, before shape/group placement and hard clipping.
//! Exact source decimals enter outward Q96 arithmetic; only final plan values
//! are quantized to Q32. No resource loading, pixel copies or host DPI defaults.
mod number;
mod regions;
mod source;
mod types;
use crate::interval::Interval as I;
use mo_geometry::{Fixed, Point};
use mo_image::{DecodedImageInfo, PhysicalPixelSize, PixelExtent};
use mo_presentation_model::Size;
use mo_presentation_source::source::fill::{
    NativeFillAlignment as Alignment, NativeTileFlip,
    resolve::{EffectiveFillRect, EffectiveImageFill, EffectiveImageMode},
};
use mo_raster::ImageTile;
use number::{positive, q32};
pub use source::{ImageSourceLayoutError, ImageSourceLayoutPlan, layout_source};
pub use types::*;

// Geometry describes the original oriented source, not a decoded sample grid.
struct ImageGeometry {
    width: u32,
    height: u32,
    density: PhysicalPixelSize,
}
impl ImageGeometry {
    fn from(image: &DecodedImageInfo) -> Result<Self, ImageLayoutError> {
        let size = image.source_size();
        if image.width == 0
            || image.height == 0
            || image.width > size.width
            || image.height > size.height
            || !(1..=8).contains(&image.orientation)
        {
            return Err(ImageLayoutError::Invalid("normalized image dimensions"));
        }
        Ok(Self {
            width: size.width,
            height: size.height,
            density: image.resolution.physical_pixel_size,
        })
    }
}

pub const PROFILE: &str = "drawingml-normalized-image-layout-q96-v1-draft";
fn cancel(check: &dyn Fn() -> bool) -> Result<(), ImageLayoutError> {
    if check() {
        Err(ImageLayoutError::Cancelled)
    } else {
        Ok(())
    }
}
fn rectangle(
    r: &EffectiveFillRect,
    size: &[I; 2],
    check: &dyn Fn() -> bool,
) -> Result<[I; 4], ImageLayoutError> {
    let mut values = Vec::with_capacity(4);
    for (i, v) in [&r.left, &r.top, &r.right, &r.bottom].iter().enumerate() {
        cancel(check)?;
        let value = number::percentage(&v.value)?;
        let value = if i < 2 {
            value
        } else {
            I::integer(1).sub(&value)
        };
        values.push(value.mul(&size[i % 2]));
    }
    let values: [I; 4] = values.try_into().expect("four rectangle edges");
    positive(&values[2].sub(&values[0]))?;
    positive(&values[3].sub(&values[1]))?;
    Ok(values)
}
fn density(
    fill: &EffectiveImageFill,
    image: &ImageGeometry,
) -> Result<([I; 2], ImageLayoutDensity), ImageLayoutError> {
    if fill.dpi.value != 0 {
        let v = I::ratio(914400, i64::from(fill.dpi.value));
        return Ok((
            [v.clone(), v],
            ImageLayoutDensity::Drawingml {
                dpi: fill.dpi.value,
            },
        ));
    }
    let PhysicalPixelSize::Known { x, y } = image.density else {
        return Err(ImageLayoutError::PhysicalSize(image.density));
    };
    let value = |v: PixelExtent| {
        if v.numerator.get() <= 0 || v.denominator == 0 {
            return Err(ImageLayoutError::Invalid("physical pixel extent"));
        }
        Ok(I::ratio(v.numerator.get(), i64::from(v.denominator)))
    };
    Ok((
        [value(x)?, value(y)?],
        ImageLayoutDensity::EncodedMetadata { x, y },
    ))
}
fn alignment(v: Alignment) -> [i64; 2] {
    use Alignment::*;
    match v {
        TopLeft => [0, 0],
        Top => [1, 0],
        TopRight => [2, 0],
        Left => [0, 1],
        Center => [1, 1],
        Right => [2, 1],
        BottomLeft => [0, 2],
        Bottom => [1, 2],
        BottomRight => [2, 2],
    }
}
/// Compute only image-local layout. The page compiler must bind the decoded
/// identity, apply the requested paint orientation, intersect shape/fill clips,
/// and propagate every returned uncertainty before producing a render command.
pub fn layout(
    fill: &EffectiveImageFill,
    image: &DecodedImageInfo,
    size: Size,
    check: &dyn Fn() -> bool,
) -> Result<NativeImageLayout, ImageLayoutError> {
    cancel(check)?;
    let (w, h) = (size.width.get(), size.height.get());
    if w <= 0 || h <= 0 {
        return Err(ImageLayoutError::Invalid("shape dimensions"));
    }
    layout_box(
        fill,
        &ImageGeometry::from(image)?,
        [I::integer(0), I::integer(0)],
        [I::integer(w), I::integer(h)],
        check,
    )
}
/// A native receiver inside the real object's coordinate system. Keep its
/// offset and uncertainty through the same crop/stretch/tile calculation;
/// neither truncate dimensions to integer EMU nor synthesize a placement.
pub(crate) fn layout_region(
    fill: &EffectiveImageFill,
    image: &DecodedImageInfo,
    bounds: mo_geometry::Rect,
    error: Fixed,
    check: &dyn Fn() -> bool,
) -> Result<NativeImageLayout, ImageLayoutError> {
    cancel(check)?;
    if error.raw() < 0 {
        return Err(ImageLayoutError::Invalid("negative receiver uncertainty"));
    }
    let enclose = |v| {
        let value = I::fixed(v);
        let error = I::fixed(error);
        I::raw(value.lo - error.lo, value.hi + error.hi)
    };
    let origin = [enclose(bounds.min.x), enclose(bounds.min.y)];
    let size = [
        enclose(bounds.max.x).sub(&origin[0]),
        enclose(bounds.max.y).sub(&origin[1]),
    ];
    layout_box(fill, &ImageGeometry::from(image)?, origin, size, check)
}
/// A unit source square gives the complete image footprint without decoding.
/// Stretch/crop geometry is independent of source density and pixel dimensions.
/// Tile layouts need the source's physical density and keep their exact grid.
pub(crate) fn unit_stretch_layout(
    fill: &EffectiveImageFill,
    size: Size,
    region: Option<(mo_geometry::Rect, Fixed)>,
    check: &dyn Fn() -> bool,
) -> Result<Option<NativeImageLayout>, ImageLayoutError> {
    if !matches!(fill.mode, EffectiveImageMode::Stretch { .. }) {
        return Ok(None);
    }
    let (origin, extent) = if let Some((r, error)) = region {
        if error.raw() < 0 {
            return Err(ImageLayoutError::Invalid("negative receiver uncertainty"));
        }
        let enclose = |v| {
            let v = I::fixed(v);
            let e = I::fixed(error);
            I::raw(v.lo - &e.lo, v.hi + &e.hi)
        };
        let origin = [enclose(r.min.x), enclose(r.min.y)];
        let extent = [
            enclose(r.max.x).sub(&origin[0]),
            enclose(r.max.y).sub(&origin[1]),
        ];
        (origin, extent)
    } else {
        (
            [I::integer(0), I::integer(0)],
            [I::integer(size.width.get()), I::integer(size.height.get())],
        )
    };
    layout_box(
        fill,
        &ImageGeometry {
            width: 1,
            height: 1,
            density: PhysicalPixelSize::Unspecified,
        },
        origin,
        extent,
        check,
    )
    .map(Some)
}
fn layout_box(
    fill: &EffectiveImageFill,
    image: &ImageGeometry,
    origin: [I; 2],
    size: [I; 2],
    check: &dyn Fn() -> bool,
) -> Result<NativeImageLayout, ImageLayoutError> {
    cancel(check)?;
    if image.width == 0 || image.height == 0 || image.width > 8192 || image.height > 8192 {
        return Err(ImageLayoutError::Invalid("normalized image dimensions"));
    }
    for dimension in &size {
        positive(dimension)?;
    }
    let source = rectangle(
        &fill.source_rect,
        &[
            I::integer(image.width.into()),
            I::integer(image.height.into()),
        ],
        check,
    )?;
    let extent = [source[2].sub(&source[0]), source[3].sub(&source[1])];
    let (target, step, tiles, clip, selected_density) = match &fill.mode {
        EffectiveImageMode::Stretch { fill_rect, .. } => {
            let r = rectangle(fill_rect, &size, check)?;
            let step = [
                r[2].sub(&r[0]).div_positive(&extent[0]),
                r[3].sub(&r[1]).div_positive(&extent[1]),
            ];
            let [x, y] = step;
            (
                r,
                [
                    x.map_err(|_| ImageLayoutError::Precision)?,
                    y.map_err(|_| ImageLayoutError::Precision)?,
                ],
                [ImageTile::Clamp; 2],
                true,
                ImageLayoutDensity::NotRequired,
            )
        }
        EffectiveImageMode::Tile { tile, .. } => {
            let (pixel, selected) = density(fill, image)?;
            let scales = [&tile.scale_x.value, &tile.scale_y.value];
            let offsets = [&tile.translate_x.value, &tile.translate_y.value];
            let align = alignment(tile.alignment.value);
            let mut steps = vec![];
            let mut edges = vec![];
            for i in 0..2 {
                cancel(check)?;
                let scale = number::percentage(scales[i])?;
                positive(&scale)?;
                let step = pixel[i].mul(&scale);
                let tile_extent = extent[i].mul(&step);
                let offset = number::coordinate(offsets[i])?;
                let lo = size[i]
                    .sub(&tile_extent)
                    .mul(&I::ratio(align[i], 2))
                    .add(&offset);
                edges.push((lo.clone(), lo.add(&tile_extent)));
                steps.push(step);
            }
            let flip = tile.flip.value;
            let x = if matches!(flip, NativeTileFlip::X | NativeTileFlip::Xy) {
                ImageTile::Mirror
            } else {
                ImageTile::Repeat
            };
            let y = if matches!(flip, NativeTileFlip::Y | NativeTileFlip::Xy) {
                ImageTile::Mirror
            } else {
                ImageTile::Repeat
            };
            (
                [
                    edges[0].0.clone(),
                    edges[1].0.clone(),
                    edges[0].1.clone(),
                    edges[1].1.clone(),
                ],
                [steps.remove(0), steps.remove(0)],
                [x, y],
                false,
                selected,
            )
        }
    };
    let target: [I; 4] = std::array::from_fn(|i| target[i].add(&origin[i % 2]));
    cancel(check)?;
    let origin_x = q32(&target[0].sub(&source[0].mul(&step[0])))?;
    let origin_y = q32(&target[1].sub(&source[1].mul(&step[1])))?;
    let (sx, ex) = q32(&step[0])?;
    let (sy, ey) = q32(&step[1])?;
    if sx.raw() <= 0 || sy.raw() <= 0 {
        return Err(ImageLayoutError::Precision);
    }
    let (source_rectangle, source_error) = number::rectangle(&source)?;
    let (fill_rectangle, target_error) = number::rectangle(&target)?;
    cancel(check)?;
    Ok(NativeImageLayout {
        profile: PROFILE.into(),
        source_rectangle,
        fill_rectangle,
        origin: Point {
            x: origin_x.0,
            y: origin_y.0,
        },
        pixel_step: Point { x: sx, y: sy },
        tile_x: tiles[0],
        tile_y: tiles[1],
        clip_to_fill_rectangle: clip,
        rotate_with_shape: fill.rotate_with_shape.value,
        density: selected_density,
        uncertainty: ImageLayoutUncertainty {
            source_rectangle: source_error,
            fill_rectangle: target_error,
            origin: Point {
                x: origin_x.1,
                y: origin_y.1,
            },
            pixel_step: Point { x: ex, y: ey },
        },
    })
}

#[cfg(test)]
mod tests;
