//! Source-bound owned resources. Requests, source index and decoded pixels are
//! immutable; each sample rebuilds/certifies world geometry and brush placement.
use super::*;
use crate::{
    source_image_layout::ImageSourceLayoutPlan, source_image_paint,
    source_placement::SourceRotations, source_text_page::retained::RetainedText,
};
use mo_common::Digest;
use mo_pptx::source::fill::resolve::FillOwner;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePreparationInfo {
    pub text_frames: u32,
    pub text_work: crate::source_frame::FrameWork,
    /// Logical paths/draw elements; not heap capacity or process RSS.
    pub text_path_bytes: u64,
    pub decoded_images: u32,
    pub decoded_pixel_bytes: u64,
    pub encoded_bytes: u64,
    pub gather_copy_bytes: u32,
    pub resources_sha256: Digest,
}
struct ImageUse {
    binding: u32,
    layout: ImageSourceLayoutPlan,
}
pub struct ResourcePagePlan {
    request: SourcePageRequest,
    index: SourceIndex,
    text_enabled: bool,
    text: Option<RetainedText>,
    images: PreparedImages<'static>,
    uses: BTreeMap<FillOwner, ImageUse>,
    decoded: Vec<mo_image::DecodedImageInfo>,
    sampling: mo_raster::ImageSampling,
    info: ResourcePreparationInfo,
}
impl ResourcePagePlan {
    /// Eagerly prepares the declared base pose. Failure publishes no owner.
    /// The raw package, font bundle, decoder and text backend may then be dropped.
    pub fn new(
        package: &dyn PackageRead,
        index: SourceIndex,
        request: SourcePageRequest,
        decoder: &mut dyn ImageDecoder,
        text: Option<TextPageContext<'_, '_, '_>>,
        options: ResourcePageOptions,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let text_enabled = text.is_some();
        let prepared = prepare(package, &index, &request, decoder, text, options, check)?;
        let PreparedResourcePage {
            built,
            text,
            images,
        } = prepared;
        let resources::Resources {
            info,
            paints: _,
            manifest,
            pixels,
        } = images;
        let images = PreparedImages::owned(&manifest, pixels, check)?;
        let compiled = mo_render::compile_images(&built.raster, &images, check)?;
        let text = text
            .map(|t| RetainedText::new(t, &built.bindings, check))
            .transpose()?;
        let mut uses = BTreeMap::new();
        for b in info.bindings {
            cancel(check)?;
            let key = FillOwner {
                part: built.bindings[b.binding as usize].location.part.clone(),
                target: b.paint.target.clone(),
            };
            if uses
                .insert(
                    key,
                    ImageUse {
                        binding: b.binding,
                        layout: b.layout,
                    },
                )
                .is_some()
            {
                return Err(SourcePageError::Invalid("duplicate retained image use"));
            }
        }
        // Preparation certifies the declared base pose.
        built.finish(
            compiled.work().clone(),
            compiled.raster().work().coordinate_error_bound,
        )?;
        drop(compiled);
        let preparation = ResourcePreparationInfo {
            text_frames: text.as_ref().map_or(0, RetainedText::frames),
            text_work: text.as_ref().map(|t| t.work.clone()).unwrap_or_default(),
            text_path_bytes: text.as_ref().map_or(0, |t| t.path_bytes),
            decoded_images: info.decoded.len() as u32,
            decoded_pixel_bytes: images.bytes().len() as u64,
            encoded_bytes: info.encoded_bytes,
            gather_copy_bytes: info.gather_copy_bytes,
            resources_sha256: images.sha256().clone(),
        };
        cancel(check)?;
        Ok(Self {
            request,
            index,
            text_enabled,
            text,
            images,
            uses,
            decoded: info.decoded,
            sampling: options.sampling,
            info: preparation,
        })
    }
    pub fn info(&self) -> &ResourcePreparationInfo {
        &self.info
    }
    pub(crate) fn render_sampled(
        &self,
        rotations: &SourceRotations,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, SourcePageError> {
        let prepared = source_page::preflight_sampled(
            &self.index,
            &self.request,
            self.text_enabled,
            true,
            Some(rotations),
            check,
        )?;
        let mut paints = BTreeMap::new();
        for (binding, owner, fill) in prepared.image_uses() {
            cancel(check)?;
            let key = FillOwner {
                part: owner.location.part.clone(),
                target: fill.target.clone(),
            };
            let usage = self
                .uses
                .get(&key)
                .filter(|u| u.binding == binding)
                .ok_or(SourcePageError::Invalid("retained image source binding"))?;
            let mut layout = usage.layout.clone();
            if layout.placement.as_ref().map(|p| p.source_size)
                != owner.placement.as_ref().map(|p| p.source_size)
            {
                return Err(SourcePageError::Invalid(
                    "retained image dimensions changed",
                ));
            }
            layout.placement = owner.placement.clone();
            let paint = source_image_paint::compile(&layout, self.sampling, check)
                .map_err(|e| SourcePageError::from(e).at(&owner.location))?;
            if paints.insert(key, paint).is_some() {
                return Err(SourcePageError::Invalid("duplicate retained image paint"));
            }
        }
        if paints.len() != self.uses.len() {
            return Err(SourcePageError::Invalid("unpainted retained image"));
        }
        let mut text = self.text.as_ref().map(RetainedText::painter);
        let built = source_page::build(
            prepared,
            text.as_mut()
                .map(|t| t as &mut dyn source_text_page::Painter),
            &paints,
            check,
        )?;
        if let Some(text) = text {
            text.finish()?;
        }
        let compiled = mo_render::compile_images(&built.raster, &self.images, check)?;
        let (info, downstream) = {
            let page = built.finish(
                compiled.work().clone(),
                compiled.raster().work().coordinate_error_bound,
            )?;
            (page.info, page.downstream_coordinate_error_bound)
        };
        drop(paints);
        let image = mo_render::render_compiled_images(compiled, backend, check)?;
        Ok(SourceResourcePageImage {
            info: SourceResourcePageRasterInfo {
                profile: PROFILE.into(),
                page: SourcePageRasterInfo {
                    page: info,
                    scene: image.info.scene,
                    downstream_coordinate_error_bound: downstream,
                },
                text_frames: self.info.text_frames,
                text_work: self
                    .text
                    .as_ref()
                    .map(RetainedText::sample_work)
                    .unwrap_or_default(),
                images: image.info.images,
                resources_sha256: image.info.resources_sha256,
                decoded_images: self.decoded.clone(),
                encoded_bytes: self.info.encoded_bytes,
                gather_copy_bytes: 0,
            },
            pixels: image.pixels,
        })
    }
}
