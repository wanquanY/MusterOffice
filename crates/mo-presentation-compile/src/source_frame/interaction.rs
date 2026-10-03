//! Text interaction retained from the same frame computation as glyph paths.
//! The private owner binds maps to native line offsets and clip constraints.
mod navigation;
mod query;
mod types;
use super::{
    SourceFrameError, SourceFrameLimits, SourceFramePlan, SourceFrameRequest, cancel,
    compute_with_interaction, prepare,
};
use mo_geometry::{Fixed, Point, Rect};
use mo_presentation_source::source::SourceIndex;
use mo_text::interaction::InteractionMap;
use mo_text::{backend::TextBackend, manifest::PreparedManifest};
pub use types::*;

pub struct SourceFrameEditor {
    pub(super) frame: SourceFramePlan,
    pub(super) maps: Vec<InteractionMap>,
    pub(super) limits: FrameInteractionLimits,
}
pub(crate) struct FrameInteraction<'a> {
    pub frame: &'a SourceFramePlan,
    pub maps: &'a [InteractionMap],
    pub limits: FrameInteractionLimits,
}
impl SourceFrameEditor {
    pub fn query(
        &self,
        queries: &[FrameTextQuery],
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<FrameTextQueryResult>, SourceFrameError> {
        FrameInteraction {
            frame: &self.frame,
            maps: &self.maps,
            limits: self.limits,
        }
        .query(queries, check)
    }
    pub fn frame(&self) -> &SourceFramePlan {
        &self.frame
    }
    pub fn paragraphs(&self) -> &[InteractionMap] {
        &self.maps
    }
    pub(crate) fn into_parts(self) -> (SourceFramePlan, Vec<InteractionMap>) {
        (self.frame, self.maps)
    }
}
/// Author and retained-source pages both compile native frames through this
/// engine. All font resources remain explicitly supplied and verified.
pub fn compile(
    index: &SourceIndex,
    q: &SourceFrameRequest,
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut dyn TextBackend,
    limits: SourceFrameLimits,
    interaction_limits: FrameInteractionLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFrameEditor, SourceFrameError> {
    let prepared = prepare(index, q, manifest, limits, check)?;
    let mut session = manifest.font_session(backend);
    compute_with_interaction(
        prepared,
        manifest,
        &mut session,
        limits,
        Some(interaction_limits),
        check,
    )
}
