//! Source-bound image resources for one prepared page. File and network
//! authority remain at the Package ReaderAt boundary; no linked URI is fetched.
use super::*;
use crate::{
    source_image_layout::{self, ImageSourceLayoutPlan},
    source_image_paint,
};
use mo_common::Digest;
use mo_opc::PartName;
use mo_presentation_source::{
    PptxError,
    source::{fill::resolve::*, images::*},
};
use std::collections::BTreeMap;

pub(super) struct Resources {
    pub info: PageImageResources,
    pub paints: BTreeMap<FillOwner, source_image_paint::NativeImagePaint>,
    pub manifest: Vec<mo_raster::ImageResource>,
    pub pixels: Vec<u8>,
}
struct Pending<'a> {
    binding_id: u32,
    owner: &'a SourcePagePaintBinding,
    target: FillTarget,
    source: SourceImageBinding,
    resource: u32,
}
pub(super) fn prepare(
    input: &dyn ImageInput,
    index: &SourceIndex,
    page: &source_page::PreparedPage<'_>,
    options: ResourcePageOptions,
    decoder: &mut dyn ImageDecoder,
    check: &dyn Fn() -> bool,
) -> Result<Resources, SourcePageError> {
    let uses = page.image_uses();
    if uses.len() > 4096 {
        return Err(RasterError::Limit("page image fills").into());
    }
    let mut groups = BTreeMap::<&str, Vec<_>>::new();
    for usage in uses {
        groups
            .entry(&usage.1.location.part)
            .or_default()
            .push(usage);
    }
    let mut pending = vec![];
    let mut descriptors = vec![];
    let mut locations = vec![];
    let mut resource_ids = BTreeMap::<(Digest, u64), u32>::new();
    let mut encoded_bytes = 0u64;
    // Finish every relationship, source-policy and orientation precondition
    // before the first decoder call. Only fills actually used by paths enter.
    for (part, uses) in groups {
        cancel(check)?;
        let catalog = mo_presentation_source::source::images::query_input_on_page(
            input,
            index,
            &SourceImageQuery {
                fill: SourceFillQuery {
                    expected_source_sha256: index.source_sha256.clone(),
                    surface: part.into(),
                    targets: uses.iter().map(|u| u.2.target.clone()).collect(),
                    profile: FillProfile::Drawingml2024DraftV1,
                },
                selection: options.selection,
            },
            part,
            &page.info.slide,
            SourceImageLimits {
                fills: FillResolveLimits {
                    max_queries: 4096,
                    ..Default::default()
                },
                ..Default::default()
            },
            check,
        )?;
        if catalog.targets.len() != uses.len() {
            return Err(SourcePageError::Invalid("page image query cardinality"));
        }
        for ((binding_id, owner, fill), result) in uses.into_iter().zip(catalog.targets) {
            cancel(check)?;
            let at = &owner.location;
            if result.target != fill.target {
                return Err(SourcePageError::Invalid("page image target binding").at(at));
            }
            let SourceImageOutcome::Available { resource, binding } = result.outcome else {
                return Err(SourcePageError::ImageResource(Box::new(result)).at(at));
            };
            let FillOutcome::Resolved {
                fill: effective,
                redirects,
            } = &fill.style
            else {
                return Err(SourcePageError::Invalid("page image fill binding").at(at));
            };
            let EffectiveFill::Image { declared_by, image } = effective.as_ref() else {
                return Err(SourcePageError::Invalid("page image fill type").at(at));
            };
            if *declared_by != binding.declared_by
                || **image != binding.image
                || *redirects != binding.redirects
            {
                return Err(SourcePageError::Invalid("page image declaring context").at(at));
            }
            // grpFill inherits fill properties (ECMA-376-1 20.1.8.35), which
            // are laid out in each receiving shape. Its declaration/resource
            // owner must not replace that shape's dimensions or placement.
            // useBgFill instead samples the background behind the receiver
            // (19.3.1.43); it still needs background replay, including alpha.
            if owner.placement.is_some()
                && redirects
                    .iter()
                    .any(|r| matches!(r.target.target, FillTarget::Background {}))
            {
                return Err(SourcePageError::Mapping {
                    location: at.clone(),
                    issue: Box::new(SourcePageIssue::FillSpace {
                        redirects: redirects.clone(),
                    }),
                });
            }
            if owner.placement.is_some() && !image.rotate_with_shape.value {
                return Err(SourcePageError::ImagePaint(
                    source_image_paint::ImagePaintError::OrientationRequired,
                )
                .at(at));
            }
            let descriptor = catalog
                .resources
                .get(resource as usize)
                .ok_or(SourcePageError::Invalid("page encoded image index"))?;
            let key = (descriptor.sha256.clone(), descriptor.byte_length.get());
            let resource = if let Some(id) = resource_ids.get(&key) {
                *id
            } else {
                if descriptors.len() >= 4096 {
                    return Err(RasterError::Limit("page image resources").into());
                }
                encoded_bytes = encoded_bytes
                    .checked_add(descriptor.byte_length.get())
                    .filter(|n| *n <= 64 * 1024 * 1024)
                    .ok_or(RasterError::Limit("page encoded image bytes"))?;
                let id = descriptors.len() as u32;
                descriptors.push(descriptor.clone());
                locations.push(at.clone());
                resource_ids.insert(key, id);
                id
            };
            pending.push(Pending {
                binding_id,
                owner,
                target: fill.target.clone(),
                source: *binding,
                resource,
            });
        }
    }
    let mut decoded = vec![];
    let mut pixel_buffers = vec![];
    let mut pixel_bytes = 0usize;
    for (id, descriptor) in descriptors.iter().enumerate() {
        cancel(check)?;
        // Descriptors were just computed from this immutable package. Recheck
        // bytes against their digest in mo_image::decode before entering FFI.
        let encoded = input.read(
            &PartName::new(&descriptor.part).map_err(PptxError::from)?,
            mo_image::MAX_ENCODED_BYTES as u64,
            check,
        )?;
        let at = &locations[id];
        let image = mo_image::decode(&encoded, &descriptor.sha256, decoder, check)
            .map_err(|e| SourcePageError::from(e).at(at))?;
        pixel_bytes = pixel_bytes
            .checked_add(image.pixels().len())
            .filter(|n| *n <= mo_image::MAX_PIXEL_BYTES)
            .ok_or(RasterError::Limit("page decoded image bytes"))?;
        let (info, pixels) = image.into_parts();
        decoded.push(info);
        pixel_buffers.push(pixels);
    }
    let mut paints = BTreeMap::new();
    let mut bindings = vec![];
    for item in pending {
        cancel(check)?;
        let at = &item.owner.location;
        let placement = item.owner.placement.clone();
        let size = placement
            .as_ref()
            .map(|p| p.source_size)
            .or(index.page_size)
            .ok_or(SourcePageError::Invalid("page image source size"))?;
        let image = &decoded[item.resource as usize];
        let layout = if let Some(region) = &item.owner.region {
            source_image_layout::layout_region(
                &item.source.image,
                image,
                region.bounds,
                region.coordinate_error_bound,
                check,
            )
        } else {
            source_image_layout::layout(&item.source.image, image, size, check)
        }
        .map_err(|e| SourcePageError::from(e).at(at))?;
        let layout = ImageSourceLayoutPlan {
            target: item.target.clone(),
            resource: item.resource,
            placement,
            layout,
        };
        let paint = source_image_paint::compile(&layout, options.sampling, check)
            .map_err(|e| SourcePageError::from(e).at(at))?;
        let key = FillOwner {
            part: at.part.clone(),
            target: item.target,
        };
        if paints.insert(key, paint.clone()).is_some() {
            return Err(SourcePageError::Invalid("duplicate page image use"));
        }
        bindings.push(PageImageBinding {
            binding: item.binding_id,
            source: item.source,
            layout,
            paint,
        });
    }
    bindings.sort_by(|a, b| (a.binding, &a.paint.target).cmp(&(b.binding, &b.paint.target)));
    let manifest = decoded
        .iter()
        .map(|info| mo_raster::ImageResource {
            width: info.width,
            height: info.height,
            alpha: mo_raster::ImageAlpha::Premultiplied,
            sha256: info.pixels_sha256.clone(),
        })
        .collect();
    // One decoded image moves without a pixel copy. Multiple distinct images
    // have one bounded gather, never one copy per placement. The retained 64 MiB
    // budget is NOT an RSS promise: decoder scratch/in-flight result and gather
    // allocations coexist temporarily and are accounted separately by hosts.
    let gather_copy_bytes;
    let pixels = if pixel_buffers.len() == 1 {
        gather_copy_bytes = 0;
        pixel_buffers.pop().expect("one decoded image")
    } else {
        gather_copy_bytes = pixel_bytes as u32;
        let mut pixels = vec![];
        pixels
            .try_reserve_exact(pixel_bytes)
            .map_err(|_| RasterError::Limit("page image gather allocation"))?;
        for buffer in pixel_buffers {
            for chunk in buffer.chunks(16384) {
                cancel(check)?;
                pixels.extend_from_slice(chunk);
            }
        }
        pixels
    };
    cancel(check)?;
    Ok(Resources {
        info: PageImageResources {
            selection: options.selection,
            sampling: options.sampling,
            encoded_bytes,
            decoded,
            bindings,
            gather_copy_bytes,
        },
        paints,
        manifest,
        pixels,
    })
}
