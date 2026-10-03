//! Source-bound owned resources. Requests, source index and decoded pixels are
//! retained; each sample rebuilds/certifies world geometry and brush placement.
use super::*;
mod viewport;
use crate::{
    source_image_layout::ImageSourceLayoutPlan, source_image_paint,
    source_placement::SourceProperties, source_text_page::retained::RetainedText,
};
use mo_common::Digest;
use mo_presentation_source::source::fill::resolve::FillOwner;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
pub(crate) use viewport::ResourceViewportUpdate;
use viewport::RetainedImages;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePreparationInfo {
    pub text_frames: u32,
    pub text_work: crate::source_frame::FrameWork,
    /// Logical paths, draw elements and optional clip metadata; not heap capacity or RSS.
    pub text_path_bytes: u64,
    pub decoded_images: u32,
    pub decoded_pixel_bytes: u64,
    pub encoded_bytes: u64,
    pub gather_copy_bytes: u32,
    pub resources_sha256: Digest,
}
struct ImageUse {
    object: Option<u32>,
    region: Option<crate::source_page::SourcePaintRegion>,
    layout: ImageSourceLayoutPlan,
}
pub struct ResourcePagePlan {
    request: SourcePageRequest,
    index: Arc<SourceIndex>,
    tables: Option<crate::source_table::RetainedTables>,
    text_enabled: bool,
    text: Option<RetainedText>,
    images: RetainedImages,
    options: ResourcePageOptions,
    info: ResourcePreparationInfo,
}
impl ResourcePagePlan {
    /// Eagerly prepares the declared base pose. Failure publishes no owner.
    /// The raw package, font bundle, decoder and text backend may then be dropped.
    /// Without a bound timing graph, arbitrary future transforms require exact grids.
    pub fn new(
        package: &dyn PackageRead,
        index: SourceIndex,
        request: SourcePageRequest,
        decoder: &mut dyn ImageDecoder,
        text: Option<TextPageContext<'_, '_, '_>>,
        options: ResourcePageOptions,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        Self::new_visible_union(
            package,
            index,
            request,
            &SourceProperties::new(),
            DecodePolicy::Exact,
            decoder,
            text,
            options,
            check,
        )
    }
    /// Prepares all resources that the admitted timing graph can reveal.
    /// The visibility union only affects preparation, never the stored index.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_visible_union(
        package: &dyn PackageRead,
        index: SourceIndex,
        request: SourcePageRequest,
        visibility: &SourceProperties,
        decode_policy: DecodePolicy<'_>,
        decoder: &mut dyn ImageDecoder,
        text: Option<TextPageContext<'_, '_, '_>>,
        options: ResourcePageOptions,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let text_enabled = text.is_some();
        let index = Arc::new(index);
        let prepared = prepare_view(
            package,
            &index,
            PageView {
                source_owner: Some(&index),
                request: &request,
                transforms: Some(visibility),
                decode_policy,
                interaction_limits: None,
            },
            decoder,
            text,
            options,
            check,
        )?;
        let PreparedResourcePage {
            tables,
            built,
            text,
            images,
            interaction: _,
        } = prepared;
        let images = RetainedImages::new(
            images,
            &built
                .bindings
                .iter()
                .enumerate()
                .map(|(i, binding)| (i as u32, binding))
                .collect(),
            check,
        )?;
        let compiled = mo_render::compile_images(&built.raster, &images.data, check)?;
        let text = text
            .map(|t| RetainedText::new(t, &built.bindings, check))
            .transpose()?;
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
            decoded_images: images.decoded.len() as u32,
            decoded_pixel_bytes: images.data.bytes().len() as u64,
            encoded_bytes: images.encoded_bytes,
            gather_copy_bytes: images.gather_copy_bytes,
            resources_sha256: images.data.sha256().clone(),
        };
        cancel(check)?;
        Ok(Self {
            request,
            index,
            tables,
            text_enabled,
            text,
            images,
            options,
            info: preparation,
        })
    }
    pub fn info(&self) -> &ResourcePreparationInfo {
        &self.info
    }
    pub(crate) fn prepare_sampled(
        &self,
        transforms: &SourceProperties,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedResourceFrame, SourcePageError> {
        self.prepare_view(&self.request, &self.images, transforms, check)
    }
    fn prepare_view(
        &self,
        request: &SourcePageRequest,
        images: &RetainedImages,
        transforms: &SourceProperties,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedResourceFrame, SourcePageError> {
        let mut prepared = source_page::preflight_retained(
            &self.index,
            request,
            self.text_enabled,
            true,
            Some(transforms),
            self.tables.as_ref(),
            check,
        )?;
        // Retained text already owns its prepared frames; source grids are no
        // longer needed after this sample's paint geometry has been compiled.
        prepared.tables.clear();
        prepared.source = None;
        let mut paints = BTreeMap::new();
        for (_, owner, fill) in prepared.image_uses() {
            cancel(check)?;
            let key = FillOwner {
                part: owner.location.part.clone(),
                target: fill.target.clone(),
            };
            let usage = images
                .uses
                .get(&key)
                .filter(|u| u.object == owner.location.object)
                .ok_or(SourcePageError::Invalid("retained image source binding"))?;
            let mut layout = usage.layout.clone();
            if layout.placement.as_ref().map(|p| p.source_size)
                != owner.placement.as_ref().map(|p| p.source_size)
                || usage.region.map(|r| (r.bounds, r.coordinate_error_bound))
                    != owner.region.map(|r| (r.bounds, r.coordinate_error_bound))
            {
                return Err(SourcePageError::Invalid(
                    "retained image dimensions changed",
                ));
            }
            layout.placement = owner.placement.clone();
            let paint = source_image_paint::compile_sampled(
                &layout,
                &images.decoded[layout.resource as usize],
                self.options.sampling,
                check,
            )
            .map_err(|e| SourcePageError::from(e).at(&owner.location))?;
            if paints.insert(key, paint).is_some() {
                return Err(SourcePageError::Invalid("duplicate retained image paint"));
            }
        }
        // Compare against visible stable source identities. Paint-array
        // ordinals change as objects disappear; they are not resource IDs.
        let visible: std::collections::BTreeSet<_> =
            std::iter::once((prepared.background.location.part.clone(), None))
                .chain(
                    prepared
                        .objects
                        .iter()
                        .map(|o| (o.binding.location.part.clone(), o.binding.location.object)),
                )
                .collect();
        let expected = images
            .uses
            .iter()
            .filter(|(key, usage)| visible.contains(&(key.part.clone(), usage.object)))
            .count();
        if paints.len() != expected {
            return Err(SourcePageError::Invalid("unpainted retained image"));
        }
        let mut text = self.text.as_ref().map(|text| text.painter(&visible));
        let built = source_page::build(
            prepared,
            text.as_mut()
                .map(|t| t as &mut dyn source_text_page::Painter),
            &paints,
            check,
        )?;
        let (text_work, text_capacity) = match text {
            Some(text) => text.finish()?,
            None => (
                Default::default(),
                crate::source_frame::capacity::TextCapacity {
                    profile: crate::source_frame::capacity::PROFILE.into(),
                    frames: vec![],
                    page_ink: Some(vec![]),
                },
            ),
        };
        let compiled =
            mo_render::compile_shared_images(&built.raster, Arc::clone(&images.data), check)?;
        let (info, downstream) = {
            let page = built.finish(
                compiled.work().clone(),
                compiled.raster().work().coordinate_error_bound,
            )?;
            (page.info, page.downstream_coordinate_error_bound)
        };
        drop(paints);
        Ok(PreparedResourceFrame {
            compiled,
            page: info,
            downstream,
            text_capacity,
            text_work,
            decoded: images.decoded.clone(),
            encoded_bytes: images.encoded_bytes,
        })
    }
}
