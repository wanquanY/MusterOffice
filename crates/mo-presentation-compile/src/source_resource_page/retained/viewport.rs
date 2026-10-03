//! Atomically replace viewport-dependent image grids while retaining source,
//! text paths and timing. A rejected candidate never touches live resources.
use super::*;
pub(super) struct RetainedImages {
    pub data: Arc<PreparedImages<'static>>,
    pub uses: BTreeMap<FillOwner, ImageUse>,
    pub decoded: Vec<mo_image::DecodedImageInfo>,
    pub encoded_bytes: u64,
    pub gather_copy_bytes: u32,
}
impl RetainedImages {
    pub(super) fn new(
        resources: resources::Resources,
        bindings: &BTreeMap<u32, &SourcePagePaintBinding>,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let resources::Resources {
            info,
            manifest,
            pixels,
            ..
        } = resources;
        let data = Arc::new(PreparedImages::owned(&manifest, pixels, check)?);
        let mut uses = BTreeMap::new();
        for b in info.bindings {
            cancel(check)?;
            let owner = bindings
                .get(&b.binding)
                .ok_or(SourcePageError::Invalid("retained image binding ordinal"))?;
            let key = FillOwner {
                part: owner.location.part.clone(),
                target: b.paint.target,
            };
            if uses
                .insert(
                    key,
                    ImageUse {
                        object: owner.location.object,
                        region: owner.region,
                        layout: b.layout,
                    },
                )
                .is_some()
            {
                return Err(SourcePageError::Invalid("duplicate retained image use"));
            }
        }
        Ok(Self {
            data,
            uses,
            decoded: info.decoded,
            encoded_bytes: info.encoded_bytes,
            gather_copy_bytes: info.gather_copy_bytes,
        })
    }
}
impl ResourcePagePlan {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_resize(
        &mut self,
        package: &dyn PackageRead,
        viewport: mo_raster::RasterViewport,
        visibility: &SourceProperties,
        decode_policy: DecodePolicy<'_>,
        decoder: &mut dyn ImageDecoder,
        check: &dyn Fn() -> bool,
    ) -> Result<ResourceViewportUpdate<'_>, SourcePageError> {
        cancel(check)?;
        viewport.validate()?;
        if package.sha256() != &self.index.source_sha256 {
            return Err(SourcePageError::SourceConflict);
        }
        let mut request = self.request.clone();
        request.viewport = viewport;
        let prepared = source_page::preflight_retained(
            &self.index,
            &request,
            self.text_enabled,
            true,
            Some(visibility),
            self.tables.as_ref(),
            check,
        )?;
        let resources = resources::prepare(
            &PackageImages(package),
            &self.index,
            &prepared,
            self.options,
            decode_policy,
            decoder,
            check,
        )?;
        let bindings = prepared
            .image_uses()
            .into_iter()
            .map(|(ordinal, owner, _)| (ordinal, owner))
            .collect();
        let images = RetainedImages::new(resources, &bindings, check)?;
        // Re-certify the resource union using the original shaped text paths.
        // No font/backend access, timeline evaluation or OOXML re-import occurs.
        self.prepare_view(&request, &images, visibility, check)?;
        let mut info = self.info.clone();
        info.decoded_images = images.decoded.len() as u32;
        info.decoded_pixel_bytes = images.data.bytes().len() as u64;
        info.encoded_bytes = images.encoded_bytes;
        info.gather_copy_bytes = images.gather_copy_bytes;
        info.resources_sha256 = images.data.sha256().clone();
        cancel(check)?;
        Ok(ResourceViewportUpdate {
            owner: self,
            images,
            info,
            request,
        })
    }
}

pub(crate) struct ResourceViewportUpdate<'a> {
    owner: &'a mut ResourcePagePlan,
    images: RetainedImages,
    info: ResourcePreparationInfo,
    request: SourcePageRequest,
}
impl ResourceViewportUpdate<'_> {
    pub(crate) fn info(&self) -> &ResourcePreparationInfo {
        &self.info
    }
    pub(crate) fn commit(self) {
        self.owner.images = self.images;
        self.owner.info = self.info;
        self.owner.request = self.request;
    }
}
