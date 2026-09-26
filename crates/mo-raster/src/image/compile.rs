use super::{ImageBrush, ImageSampling, ImageTile, PreparedImages, precision};
use crate::{PixelScale, RasterError, RasterViewport, number::Scale};
use mo_geometry::Fixed;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct ImageBrushes {
    pub words: Vec<[u32; 14]>,
    indices: BTreeMap<[u32; 14], u32>,
    pub coordinate_error: Fixed,
    pub has_domains: bool,
}
impl ImageBrushes {
    pub fn insert(
        &mut self,
        brush: &ImageBrush,
        images: &PreparedImages<'_>,
        view: &RasterViewport,
    ) -> Result<u32, RasterError> {
        let resource = images
            .descriptors
            .get(brush.resource as usize)
            .ok_or(RasterError::Invalid("image resource reference"))?;
        let (errors, mut domain_errors) = precision::input_errors(brush, view)?;
        let (matrix, errors) =
            crate::paint_matrix::compile(brush.origin, brush.x_step, brush.y_step, errors, view)?;
        crate::paint_matrix::bounds(
            matrix,
            [0.0, 0.0, f64::from(resource[1]), f64::from(resource[2])],
        )?;
        let mut domain = [0.0, 0.0, resource[1] as f32, resource[2] as f32];
        let mut affine_extent = [i128::from(resource[1]), i128::from(resource[2])];
        if let Some(rect) = brush.source_domain {
            self.has_domains = true;
            if rect.right <= rect.left || rect.bottom <= rect.top {
                return Err(RasterError::Invalid("empty image source domain"));
            }
            let unit = Scale::new(PixelScale {
                numerator: 1,
                denominator: 1,
            })?;
            for (i, value) in [rect.left, rect.top, rect.right, rect.bottom]
                .iter()
                .enumerate()
            {
                domain[i] = unit.value(value.raw())?;
                domain_errors[i] = domain_errors[i]
                    .checked_add(unit.error(value.raw(), domain[i])?)
                    .ok_or(RasterError::Range)?;
                // Bound against the exact input, not the rounded float edge:
                // rounding just above an integer down must not shrink this box.
                affine_extent[i % 2] =
                    affine_extent[i % 2].max((value.raw().abs() + (1i128 << 32) - 1) >> 32);
            }
            if domain[2] - domain[0] < 1.0 / 16384.0 || domain[3] - domain[1] < 1.0 / 16384.0 {
                return Err(RasterError::Precision);
            }
        }
        let d = domain.map(f64::from);
        self.coordinate_error = self.coordinate_error.max(crate::paint_precision::bound(
            matrix,
            errors,
            domain,
            domain_errors,
            affine_extent,
            [brush.tile_x, brush.tile_y]
                .map(|v| matches!(v, ImageTile::Repeat | ImageTile::Mirror)),
            view,
        )?);
        crate::paint_matrix::bounds(matrix, d)?;
        let tile = |v| match v {
            ImageTile::Clamp => 0,
            ImageTile::Repeat => 1,
            ImageTile::Mirror => 2,
            ImageTile::Decal => 3,
        };
        let mut words = [0; 14];
        words[..4].copy_from_slice(&[
            brush.resource,
            tile(brush.tile_x),
            tile(brush.tile_y),
            u32::from(brush.sampling == ImageSampling::Linear),
        ]);
        for (out, value) in words[4..10].iter_mut().zip(matrix) {
            *out = value.to_bits();
        }
        for (out, value) in words[10..].iter_mut().zip(domain) {
            *out = value.to_bits();
        }
        if let Some(index) = self.indices.get(&words) {
            return Ok(*index);
        }
        if self.words.len() == 4096 {
            return Err(RasterError::Limit("image brushes"));
        }
        let index = self.words.len() as u32 + 1;
        self.words.push(words);
        self.indices.insert(words, index);
        Ok(index)
    }
}
