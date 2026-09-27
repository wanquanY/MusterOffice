//! Immutable document/timing plan feeding the existing certified page pipeline.
use crate::{PageError, PagePlacements, PagePlan, PageRasterInfo, PageRenderRequest, angle::Angle};
use mo_common::RationalTime;
use mo_raster::RasterBackend;
use mo_timeline::{
    EvaluatedFrame, EventHistory, PlaybackBinding, Timeline, TimelineError, TimelineLimits,
    TimelinePlan, TimelineSampler, TimelineSamplerInfo, TimelineVersion,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const PLAYBACK_PAGE_PROFILE: &str = "rotation-playback-author-page-q96-v1-draft";
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error(transparent)]
    Time(#[from] TimelineError),
    #[error(transparent)]
    Page(#[from] PageError),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackCompiledFrame {
    pub profile: String,
    pub frame: EvaluatedFrame,
    pub placements: PagePlacements,
    pub page: PagePlan,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackRasterInfo {
    pub profile: String,
    pub frame: EvaluatedFrame,
    pub page: PageRasterInfo,
}
pub struct PlaybackImage {
    pub info: PlaybackRasterInfo,
    pub pixels: Vec<u8>,
}
/// One evaluated frame and its privately compiled geometry. It owns its data;
/// dropping it before execution cancels without invoking any component.
/// Session owners must additionally fence completion against their live binding.
pub struct PreparedPlaybackFrame {
    frame: EvaluatedFrame,
    page: crate::PagePlanInfo,
    combined: mo_geometry::Fixed,
    compiled: mo_render::CompiledScene,
}
impl PreparedPlaybackFrame {
    pub fn binding(&self) -> &PlaybackBinding {
        &self.frame.state.binding
    }
    pub fn raster(&self) -> &mo_raster::CompiledRaster {
        self.compiled.raster()
    }
    pub fn complete(
        self,
        reply: mo_raster::BackendReply,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackImage, PlaybackError> {
        let image = self
            .compiled
            .complete(reply, check)
            .map_err(PageError::from)?;
        Ok(finish_frame(self.frame, self.page, self.combined, image))
    }
    pub fn render(
        self,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackImage, PlaybackError> {
        let image =
            mo_render::render_compiled(self.compiled, backend, check).map_err(PageError::from)?;
        Ok(finish_frame(self.frame, self.page, self.combined, image))
    }
}
fn finish_frame(
    frame: EvaluatedFrame,
    page: crate::PagePlanInfo,
    combined: mo_geometry::Fixed,
    image: mo_render::SceneImage,
) -> PlaybackImage {
    PlaybackImage {
        info: PlaybackRasterInfo {
            profile: PLAYBACK_PAGE_PROFILE.into(),
            frame,
            page: PageRasterInfo {
                page,
                scene: image.info,
                combined_coordinate_error_bound: combined,
            },
        },
        pixels: image.pixels,
    }
}
/// Holds author data and one interval schedule; each sample derives transient
/// property/placement data.
/// Geometry and scene lowering still run per sampled frame. This is not yet the
/// complete retained compositor, host state machine or media session.
pub struct PlaybackPagePlan {
    request: PageRenderRequest,
    base: PagePlacements,
    timeline: TimelineSampler,
    binding: PlaybackBinding,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GenerationError {
    #[error("playback binding does not match the prepared plan")]
    BindingConflict,
    #[error("playback generation must increase without wrapping")]
    NotIncreasing,
}
impl PlaybackPagePlan {
    pub fn new(
        request: PageRenderRequest,
        binding: PlaybackBinding,
        limits: TimelineLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PlaybackError> {
        let base = crate::page_placements(&request.page, check).map_err(PageError::from)?;
        let empty = Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![],
        };
        let timeline = TimelinePlan::compile(
            request
                .page
                .document
                .timelines
                .get(&request.page.slide)
                .unwrap_or(&empty),
            limits,
            check,
        )?;
        Ok(Self {
            request,
            base,
            timeline: TimelineSampler::new(timeline),
            binding,
        })
    }
    pub fn document_sha256(&self) -> &mo_common::Digest {
        &self.base.document_sha256
    }
    pub fn binding(&self) -> &PlaybackBinding {
        &self.binding
    }
    pub fn timing_info(&self) -> TimelineSamplerInfo {
        self.timeline.info()
    }
    /// Fences old samples without modifying the document or rebuilding its plan.
    /// Host-supplied event history for the new generation must be explicit.
    pub fn advance_generation(
        &mut self,
        expected: &PlaybackBinding,
        next: mo_timeline::PlaybackGeneration,
    ) -> Result<(), GenerationError> {
        if expected != &self.binding {
            return Err(GenerationError::BindingConflict);
        }
        if next.get() <= expected.generation.get() {
            return Err(GenerationError::NotIncreasing);
        }
        self.timeline.clear();
        self.binding.generation = next;
        Ok(())
    }
    fn sample(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<(EvaluatedFrame, Option<PagePlacements>), PlaybackError> {
        let frame = self.timeline.evaluate(&self.binding, at, history, check)?;
        if frame.state.rotations.is_empty() {
            return Ok((frame, None));
        }
        let rotations: BTreeMap<_, _> = frame
            .state
            .rotations
            .iter()
            .map(|(id, v)| {
                crate::cancel(check)?;
                Ok((id.clone(), Angle::exact(v)?))
            })
            .collect::<Result<_, crate::CompileError>>()
            .map_err(PageError::from)?;
        let placements = crate::placement::place_validated(
            &self.request.page,
            &self.base.document_sha256,
            &rotations,
            check,
        )
        .map_err(PageError::from)?;
        Ok((frame, Some(placements)))
    }
    pub fn compile_frame(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackCompiledFrame, PlaybackError> {
        let (frame, placements) = self.sample(at, history, check)?;
        let (page, _) = crate::page::prepare_page(
            &self.request,
            Some(placements.as_ref().unwrap_or(&self.base)),
            check,
        )?;
        Ok(PlaybackCompiledFrame {
            profile: PLAYBACK_PAGE_PROFILE.into(),
            frame,
            placements: placements.unwrap_or_else(|| self.base.clone()),
            page,
        })
    }
    pub fn render_frame(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackImage, PlaybackError> {
        self.prepare_frame(at, history, check)?
            .render(backend, check)
    }
    pub fn prepare_frame(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedPlaybackFrame, PlaybackError> {
        let (frame, placements) = self.sample(at, history, check)?;
        let (page, compiled) = crate::page::prepare_page(
            &self.request,
            Some(placements.as_ref().unwrap_or(&self.base)),
            check,
        )?;
        Ok(PreparedPlaybackFrame {
            frame,
            page: page.info,
            combined: page.combined_coordinate_error_bound,
            compiled,
        })
    }
}
