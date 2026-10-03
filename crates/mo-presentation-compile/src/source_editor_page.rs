//! Immutable editing geometry captured with the actual resource-enabled page.
//! Source ownership, fonts, decoding, layer order and raster remain shared.
mod query;
mod transform;
mod types;
use crate::source_frame::interaction::FrameInteractionLimits;
use crate::{source_page::*, source_resource_page::*, source_text_page::TextPageInteraction};
use mo_image::ImageDecoder;
use mo_presentation_source::source::{SourceIndex, images::ImageInput};
use mo_raster::RasterBackend;
pub use types::*;

pub struct PreparedEditorPage {
    pub(crate) inner: PreparedResourcePage,
}
/// One immutable source/page/viewport result. Client geometry cannot construct
/// an owner or pair interaction maps with a different rendered page.
pub struct SourceEditorPage {
    page: SourceResourcePagePlan,
    interaction: TextPageInteraction,
}
pub struct EditorPageImage {
    pub page: SourceEditorPage,
    pub pixels: Vec<u8>,
}
#[derive(Clone, Copy)]
pub struct EditorPageOptions {
    pub resources: ResourcePageOptions,
    pub interaction: FrameInteractionLimits,
}
pub fn prepare(
    input: &dyn ImageInput,
    index: &SourceIndex,
    q: &SourcePageRequest,
    decoder: &mut dyn ImageDecoder,
    text: TextPageContext<'_, '_, '_>,
    options: EditorPageOptions,
    check: &dyn Fn() -> bool,
) -> Result<PreparedEditorPage, SourcePageError> {
    let inner = crate::source_resource_page::prepare_input_view(
        input,
        index,
        crate::source_resource_page::PageView {
            source_owner: None,
            request: q,
            transforms: None,
            decode_policy: crate::source_resource_page::DecodePolicy::Viewport,
            interaction_limits: Some(options.interaction),
        },
        decoder,
        Some(text),
        options.resources,
        check,
    )?;
    Ok(PreparedEditorPage { inner })
}
impl PreparedEditorPage {
    pub fn plan(self, check: &dyn Fn() -> bool) -> Result<SourceEditorPage, SourcePageError> {
        self.inner.editor_plan(check)
    }
    pub fn render(
        self,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<EditorPageImage, SourcePageError> {
        self.inner.editor_render(backend, check)
    }
}
impl SourceEditorPage {
    pub(crate) fn new(
        page: SourceResourcePagePlan,
        interaction: TextPageInteraction,
    ) -> Result<Self, SourcePageError> {
        if page
            .text
            .as_ref()
            .is_none_or(|text| text.texts.len() != interaction.maps.len())
        {
            return Err(SourcePageError::Invalid("editor page interaction binding"));
        }
        Ok(Self { page, interaction })
    }
    pub fn page(&self) -> &SourceResourcePagePlan {
        &self.page
    }
    pub fn paragraphs(&self, frame: usize) -> Option<&[mo_text::interaction::InteractionMap]> {
        self.interaction.maps.get(frame).map(Vec::as_slice)
    }
}
