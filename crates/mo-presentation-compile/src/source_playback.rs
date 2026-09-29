//! Native slide timing feeds the existing immutable resource-page pipeline.
//! Source declarations stay original; sampled properties affect frame composition.
use crate::{source_page::*, source_placement::SourceProperties, source_resource_page::*};
use mo_common::{Digest, ObjectId, RationalTime};
use mo_opc::PackageRead;
use mo_presentation_source::{
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
pub const TRANSFORM_PROFILE: &str = "source-transform-resource-page-q96-v1-draft";
pub const MOTION_PROFILE: &str = "source-motion-resource-page-q96-v1-draft";
pub const PROPERTY_PROFILE: &str = "source-property-resource-page-q96-v1-draft";
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
impl SourcePlaybackFrame {
    pub fn profile(&self) -> &'static str {
        if self.evaluated.state.profile == mo_timeline::PACED_MOTION_FRAME_PROFILE {
            "source-paced-motion-resource-page-q64-v1-draft"
        } else if self.evaluated.state.profile == mo_timeline::MOTION_FRAME_PROFILE {
            MOTION_PROFILE
        } else if self.evaluated.state.profile == mo_timeline::PROPERTY_FRAME_PROFILE {
            PROPERTY_PROFILE
        } else if self.evaluated.state.profile == mo_timeline::TRANSFORM_FRAME_PROFILE {
            TRANSFORM_PROFILE
        } else {
            PROFILE
        }
    }
}
pub struct SourcePlaybackPlan {
    page: SourcePageRequest,
    binding: PlaybackBinding,
    part_sha256: Digest,
    objects: BTreeMap<ObjectId, SourceObjectRef>,
    timeline: TimelineSampler,
    /// Visible assignments may expose originally hidden content. Prepare the
    /// union once, without admitting unrelated permanently hidden resources.
    resource_visibility: SourceProperties,
}
/// Not constructible from a host-supplied property map. It borrows the immutable
/// timing/page plan that evaluated and bound these exact transforms.
pub struct SourcePlaybackSample<'a> {
    plan: &'a SourcePlaybackPlan,
    frame: SourcePlaybackFrame,
    transforms: SourceProperties,
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
        let page = ResourcePagePlan::new_visible_union(
            package,
            index,
            self.page.clone(),
            &self.resource_visibility,
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
            .map(|o| (o.native_id, (o.kind, o.table.is_some())))
            .collect();
        for node in &timeline.nodes {
            let owner = &objects[node.target()];
            let (kind, table) = kinds[&owner.native_id];
            if !(matches!(
                kind,
                SourceObjectKind::Shape
                    | SourceObjectKind::Picture
                    | SourceObjectKind::Group
                    | SourceObjectKind::Connector
            ) || kind == SourceObjectKind::GraphicFrame && table)
            {
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
        let mut resource_visibility = SourceProperties::new();
        for node in &timeline.nodes {
            if check() {
                return Err(TimelineError::Cancelled.into());
            }
            if matches!(
                node.effect,
                mo_timeline::Effect::SetVisibility {
                    value: mo_timeline::Visibility::Visible,
                    ..
                }
            ) {
                let owner = &objects[node.target()];
                resource_visibility.insert(
                    (owner.part.clone(), owner.native_id),
                    crate::sampled_properties::SampledProperties {
                        visibility: Some(mo_timeline::Visibility::Visible),
                        ..Default::default()
                    },
                );
            }
        }
        let timeline = TimelinePlan::compile(&timeline, timing_limits, check)?;
        Ok(Self {
            page,
            binding,
            part_sha256: timing.part_sha256,
            objects,
            timeline: TimelineSampler::new(timeline),
            resource_visibility,
        })
    }
    pub fn sample(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourcePlaybackSample<'_>, SourcePlaybackError> {
        let evaluated = self.timeline.evaluate(&self.binding, at, history, check)?;
        let values = crate::sampled_properties::sampled(&evaluated.state, check).map_err(|e| {
            SourcePageError::Placement(match e {
                crate::CompileError::Limit(reason) => {
                    crate::source_placement::SourcePlacementError::Limit(reason)
                }
                crate::CompileError::Cancelled => {
                    crate::source_placement::SourcePlacementError::Cancelled
                }
                _ => crate::source_placement::SourcePlacementError::Invalid(
                    "sampled transform value",
                ),
            })
        })?;
        let mut transforms = SourceProperties::new();
        for (id, value) in values {
            if check() {
                return Err(TimelineError::Cancelled.into());
            }
            let owner = &self.objects[&id];
            transforms.insert((owner.part.clone(), owner.native_id), value);
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
            transforms,
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
        let (frame, page) = self.prepare_frame(at, history, check)?;
        Ok((frame, page.render(backend, check)?))
    }
    pub fn prepare_frame(
        &mut self,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<(SourcePlaybackFrame, PreparedResourceFrame), SourcePlaybackError> {
        let sample = self.timing.sample(at, history, check)?;
        let page = self.page.prepare_sampled(&sample.transforms, check)?;
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
                source_owner: None,
                request: &self.plan.page,
                transforms: Some(&self.transforms),
            },
            decoder,
            text,
            options,
            check,
        )
    }
}
