use super::*;
use mo_common::ObjectId;
use mo_opc::Package;
use mo_pptx::{
    AuthorPlan, PptxError,
    source::{
        SourceIndex,
        document::SourcePlan,
        images::{AuthorImages, ImageInput, PackageImages, SourceImages},
    },
};
use mo_presentation_compile::{
    source_editor_page::{self, EditorPageImage, EditorPageOptions},
    source_page::SourcePageError,
    source_resource_page::{ResourcePageOptions, TextPageContext},
};
use mo_text::manifest::PreparedManifest;
use std::collections::BTreeMap;
type ObjectIds = BTreeMap<(String, u32), ObjectId>;
fn source_error(e: PptxError) -> PptxResourcePageFailure {
    SourcePageError::Source(e).into()
}
pub(super) fn prepare(
    request: &EditorPagePreparation,
    material: &[u8],
    fonts: &[u8],
    backends: EditorPageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> Result<(EditorPageImage, EditorPageInfo), PptxResourcePageFailure> {
    let (image, ids) = match &request.input {
        EditorPageInput::Pptx {} => {
            let prepared = crate::prepare_pptx_resource_document_inputs(
                &request.page.page.expected_source_sha256,
                Some(&request.fonts),
                material,
                material.len() as u64,
                fonts,
                check,
            )?;
            let image = render(
                request,
                &PackageImages(&prepared.package),
                &prepared.index,
                prepared.manifest.as_ref().expect("explicit manifest"),
                backends,
                check,
            )?;
            (image, BTreeMap::new())
        }
        EditorPageInput::Author {
            document,
            defaults,
            resources,
        } => {
            let plan = AuthorPlan::new(document, defaults, Default::default(), check)
                .map_err(source_error)?;
            if plan.identity() != &request.page.page.expected_source_sha256 {
                return Err(SourcePageError::SourceConflict.into());
            }
            let bundle =
                crate::pptx_resource_page::author::bundle(resources, &plan, material, check)?;
            let images = AuthorImages::new(&plan, &bundle, check).map_err(source_error)?;
            let manifest = manifest(request, fonts, check)?;
            let image = render(
                request,
                &images,
                plan.declarations(),
                &manifest,
                backends,
                check,
            )?;
            let native_ids: BTreeMap<_, _> = plan
                .bindings()
                .object_ids
                .iter()
                .map(|(id, native)| (*native, id))
                .collect();
            let mut ids = BTreeMap::new();
            for text in &image
                .page
                .page()
                .text
                .as_ref()
                .expect("editor text context")
                .texts
            {
                cancelled(check)?;
                let object = &text.frame.text.object;
                if let Some(id) = native_ids.get(&object.native_id) {
                    ids.insert((object.part.clone(), object.native_id), (*id).clone());
                }
            }
            (image, ids)
        }
        EditorPageInput::Retained { document } => {
            let package = Package::open(
                material,
                material.len() as u64,
                crate::pptx_source::inline_limits().package,
                check,
            )
            .map_err(PptxError::from)
            .map_err(source_error)?;
            let plan = SourcePlan::new(
                document,
                &package,
                crate::pptx_source::inline_limits(),
                check,
            )
            .map_err(source_error)?;
            if plan.identity() != &request.page.page.expected_source_sha256 {
                return Err(SourcePageError::SourceConflict.into());
            }
            let images = SourceImages::new(&plan, &package).map_err(source_error)?;
            let manifest = manifest(request, fonts, check)?;
            let image = render(
                request,
                &images,
                plan.declarations(),
                &manifest,
                backends,
                check,
            )?;
            let mut ids = BTreeMap::new();
            for (id, binding) in &document
                .source_bindings
                .as_ref()
                .expect("validated source plan")
                .objects
            {
                cancelled(check)?;
                ids.insert((binding.part.clone(), binding.native_id), id.clone());
            }
            (image, ids)
        }
    };
    let info = info(&image.page, &ids, &request.page.page.viewport, check)?;
    Ok((image, info))
}
fn manifest<'a>(
    request: &'a EditorPagePreparation,
    fonts: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<PreparedManifest<'a, 'a>, PptxResourcePageFailure> {
    PreparedManifest::load(&request.fonts, fonts, Default::default(), check).map_err(|e| {
        PptxResourcePageFailure::Fonts {
            error: crate::text::shape_failure(e),
        }
    })
}
fn render(
    request: &EditorPagePreparation,
    images: &dyn ImageInput,
    index: &SourceIndex,
    manifest: &PreparedManifest<'_, '_>,
    backends: EditorPageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> Result<EditorPageImage, PptxResourcePageFailure> {
    let prepared = source_editor_page::prepare(
        images,
        index,
        &request.page.page,
        backends.decoder,
        TextPageContext {
            manifest,
            backend: backends.text,
        },
        EditorPageOptions {
            resources: ResourcePageOptions {
                selection: request.page.image_source,
                sampling: request.page.sampling,
                text_limits: Default::default(),
            },
            interaction: Default::default(),
        },
        check,
    )?;
    prepared.render(backends.raster, check).map_err(Into::into)
}
fn info(
    page: &SourceEditorPage,
    ids: &ObjectIds,
    viewport: &mo_raster::RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<EditorPageInfo, PptxResourcePageFailure> {
    let plan = page.page();
    let text = plan.text.as_ref().expect("editor text context");
    let mut frames = Vec::with_capacity(text.texts.len());
    for (frame, binding) in text.texts.iter().enumerate() {
        cancelled(check)?;
        let object = &binding.frame.text.object;
        let mut paragraphs = Vec::with_capacity(binding.frame.inputs.len());
        for (input, map) in binding
            .frame
            .inputs
            .iter()
            .zip(page.paragraphs(frame).expect("editor maps"))
        {
            cancelled(check)?;
            paragraphs.push(EditorParagraphInfo {
                text: input.text.clone(),
                source_ordinal: input.source_ordinal,
                boundaries: map.boundaries.clone(),
            });
        }
        frames.push(EditorTextFrameInfo {
            frame: frame as u32,
            object: object.clone(),
            object_id: ids.get(&(object.part.clone(), object.native_id)).cloned(),
            cell: binding.frame.text.cell,
            paragraphs,
        });
    }
    Ok(EditorPageInfo {
        page: plan.page.info.clone(),
        // The scene has already reserved part of this tolerance for upstream
        // placement. The public view describes the admitted host viewport;
        // internal remaining raster precision is not a new viewport revision.
        viewport: viewport.clone(),
        resources_sha256: plan.resources_sha256.clone(),
        text_work: text.text_work.clone(),
        text_frames: frames,
        downstream_coordinate_error_bound: plan.page.downstream_coordinate_error_bound,
    })
}
