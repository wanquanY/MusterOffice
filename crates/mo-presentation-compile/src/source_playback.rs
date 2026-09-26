//! Native slide timing feeds the existing immutable resource-page pipeline.
//! Source declarations stay original; sampled properties only affect placement.
use crate::{
    angle::Angle, source_page::*, source_placement::SourceRotations, source_resource_page::*,
};
use mo_common::{Digest, ObjectId, RationalTime};
use mo_opc::PackageRead;
use mo_pptx::{
    source::{SourceIndex, SourceLimits, SourceObjectKind, SourceObjectRef},
    timing::{self, SourceTimingQuery},
};
use mo_timeline::{
    EvaluatedFrame, EventHistory, PlaybackBinding, Timeline, TimelineError, TimelineLimits,
    TimelinePlan, TimelineSampler, TimelineSamplerInfo, TimelineVersion,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROFILE: &str = "source-rotation-resource-page-q96-v1-draft";
#[derive(Debug, thiserror::Error)]
pub enum SourcePlaybackError {
    #[error(transparent)]
    Page(#[from] SourcePageError),
    #[error(transparent)]
    Timeline(#[from] TimelineError),
    #[error("source playback revision must identify the immutable source package")]
    RevisionConflict,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePlaybackFrame {
    pub source_sha256: Digest,
    pub slide: String,
    pub part_sha256: Digest,
    /// Synthetic time-graph keys are scoped to this source slide. Never resolve
    /// a bare native id against a master, layout or another slide.
    pub object_bindings: BTreeMap<ObjectId, SourceObjectRef>,
    pub evaluated: EvaluatedFrame,
}
pub struct SourcePlaybackPlan {
    page: SourcePageRequest,
    binding: PlaybackBinding,
    part_sha256: Digest,
    objects: BTreeMap<ObjectId, SourceObjectRef>,
    timeline: TimelineSampler,
}
/// Not constructible from a host-supplied property map. It borrows the immutable
/// timing/page plan that evaluated and bound these exact rotations.
pub struct SourcePlaybackSample<'a> {
    plan: &'a SourcePlaybackPlan,
    frame: SourcePlaybackFrame,
    rotations: SourceRotations,
}
impl SourcePlaybackPlan {
    pub fn binding(&self) -> &PlaybackBinding {
        &self.binding
    }
    pub fn timing_info(&self) -> TimelineSamplerInfo {
        self.timeline.info()
    }
    pub fn advance_generation(
        &mut self,
        expected: &PlaybackBinding,
        next: mo_timeline::PlaybackGeneration,
    ) -> Result<(), crate::playback::GenerationError> {
        if expected != &self.binding {
            return Err(crate::playback::GenerationError::BindingConflict);
        }
        if next.get() <= expected.generation.get() {
            return Err(crate::playback::GenerationError::NotIncreasing);
        }
        self.timeline.clear();
        self.binding.generation = next;
        Ok(())
    }
    pub fn retain(
        self,
        package: &dyn PackageRead,
        index: SourceIndex,
        decoder: &mut dyn mo_image::ImageDecoder,
        text: Option<TextPageContext<'_, '_, '_>>,
        options: ResourcePageOptions,
        check: &dyn Fn() -> bool,
    ) -> Result<RetainedSourcePlaybackPlan, SourcePageError> {
        let page = ResourcePagePlan::new(
            package,
            index,
            self.page.clone(),
            decoder,
            text,
            options,
            check,
        )?;
        Ok(RetainedSourcePlaybackPlan { timing: self, page })
    }
    pub fn new(
        package: &dyn PackageRead,
        index: &SourceIndex,
        page: SourcePageRequest,
        binding: PlaybackBinding,
        limits: SourceLimits,
        timing_limits: TimelineLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePlaybackError> {
        if binding.revision != page.expected_source_sha256 {
            return Err(SourcePlaybackError::RevisionConflict);
        }
        let timing = timing::query_index(
            package,
            index,
            &SourceTimingQuery {
                expected_source_sha256: page.expected_source_sha256.clone(),
                slide: page.slide.clone(),
            },
            limits,
            timing_limits,
            check,
        )
        .map_err(SourcePageError::from)?;
        let (timeline, objects) = match timing.native {
            Some(native) => (
                native.timeline,
                native
                    .object_bindings
                    .into_iter()
                    .map(|(key, id)| {
                        (
                            key,
                            SourceObjectRef {
                                part: page.slide.clone(),
                                native_id: id,
                            },
                        )
                    })
                    .collect(),
            ),
            None => (
                Timeline {
                    format: TimelineVersion::V01,
                    tree: None,
                    nodes: vec![],
                },
                BTreeMap::new(),
            ),
        };
        let kinds: BTreeMap<_, _> = index.surfaces[&page.slide]
            .objects
            .iter()
            .map(|o| (o.native_id, o.kind))
            .collect();
        for node in &timeline.nodes {
            let mo_timeline::Effect::Rotation { target, .. } = &node.effect;
            let owner = &objects[target];
            let kind = kinds[&owner.native_id];
            if !matches!(
                kind,
                SourceObjectKind::Shape
                    | SourceObjectKind::Picture
                    | SourceObjectKind::Group
                    | SourceObjectKind::Connector
            ) {
                return Err(SourcePageError::Mapping {
                    location: SourcePageLocation {
                        part: owner.part.clone(),
                        object: Some(owner.native_id),
                    },
                    issue: Box::new(SourcePageIssue::Object { native_kind: kind }),
                }
                .into());
            }
        }
        let timeline = TimelinePlan::compile(&timeline, timing_limits, check)?;
        Ok(Self {
            page,
            binding,
            part_sha256: timing.part_sha256,
            objects,
            timeline: TimelineSampler::new(timeline),
        })
    }
    pub fn sample(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourcePlaybackSample<'_>, SourcePlaybackError> {
        let evaluated = self.timeline.evaluate(&self.binding, at, history, check)?;
        let mut rotations = SourceRotations::new();
        for (id, value) in &evaluated.state.rotations {
            if check() {
                return Err(TimelineError::Cancelled.into());
            }
            let owner = &self.objects[id];
            let angle = Angle::exact(value).map_err(|e| {
                SourcePageError::Placement(match e {
                    crate::CompileError::Limit(reason) => {
                        crate::source_placement::SourcePlacementError::Limit(reason)
                    }
                    crate::CompileError::Cancelled => {
                        crate::source_placement::SourcePlacementError::Cancelled
                    }
                    _ => crate::source_placement::SourcePlacementError::Invalid(
                        "sampled rotation value",
                    ),
                })
            })?;
            rotations.insert((owner.part.clone(), owner.native_id), angle);
        }
        Ok(SourcePlaybackSample {
            plan: self,
            frame: SourcePlaybackFrame {
                source_sha256: self.page.expected_source_sha256.clone(),
                slide: self.page.slide.clone(),
                part_sha256: self.part_sha256.clone(),
                object_bindings: self.objects.clone(),
                evaluated,
            },
            rotations,
        })
    }
}
/// Immutable source index, time graph and local resources; no self references,
/// host component handles, font/package byte borrows or frame cache. Timing retains
/// only one consumed event prefix and its intervals, not future events.
pub struct RetainedSourcePlaybackPlan {
    timing: SourcePlaybackPlan,
    page: ResourcePagePlan,
}
impl RetainedSourcePlaybackPlan {
    pub fn binding(&self) -> &PlaybackBinding {
        self.timing.binding()
    }
    pub fn timing_info(&self) -> TimelineSamplerInfo {
        self.timing.timing_info()
    }
    pub fn preparation(&self) -> &ResourcePreparationInfo {
        self.page.info()
    }
    pub fn advance_generation(
        &mut self,
        expected: &PlaybackBinding,
        next: mo_timeline::PlaybackGeneration,
    ) -> Result<(), crate::playback::GenerationError> {
        self.timing.advance_generation(expected, next)
    }
    pub fn render(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        backend: &mut dyn mo_raster::RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<(SourcePlaybackFrame, SourceResourcePageImage), SourcePlaybackError> {
        let sample = self.timing.sample(at, history, check)?;
        let page = self
            .page
            .render_sampled(&sample.rotations, backend, check)?;
        Ok((sample.frame, page))
    }
}
impl SourcePlaybackSample<'_> {
    pub fn frame(&self) -> &SourcePlaybackFrame {
        &self.frame
    }
    pub fn prepare(
        &self,
        package: &dyn PackageRead,
        index: &SourceIndex,
        decoder: &mut dyn mo_image::ImageDecoder,
        text: Option<TextPageContext<'_, '_, '_>>,
        options: ResourcePageOptions,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedResourcePage, SourcePageError> {
        // The shared preflight verifies both source identities again. A sample
        // cannot be replayed against a different imported package.
        super::source_resource_page::prepare_view(
            package,
            index,
            super::source_resource_page::PageView {
                request: &self.plan.page,
                rotations: Some(&self.rotations),
            },
            decoder,
            text,
            options,
            check,
        )
    }
}
