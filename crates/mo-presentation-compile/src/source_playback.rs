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
use std::collections::{BTreeMap, BTreeSet};

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
    /// Includes descendants of animated groups; resource dedup takes the strictest use.
    geometry_owners: BTreeSet<(String, u32)>,
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
            DecodePolicy::Retained(&self.geometry_owners),
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
        let mut geometry_roots = Vec::new();
        for node in &timeline.nodes {
            if check() {
                return Err(TimelineError::Cancelled.into());
            }
            if matches!(
                node.effect,
                mo_timeline::Effect::Scale { .. } | mo_timeline::Effect::Rotation { .. }
            ) {
                geometry_roots.push(objects[node.target()].native_id);
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
        // Expand group ownership once in linear graph work. Translation, visibility
        // and opacity do not enlarge texel footprints and can retain sampled grids.
        let mut children = BTreeMap::<u32, Vec<u32>>::new();
        for object in &index.surfaces[&page.slide].objects {
            if check() {
                return Err(TimelineError::Cancelled.into());
            }
            if let Some(parent) = object.parent_group {
                children.entry(parent).or_default().push(object.native_id);
            }
        }
        let mut geometry_owners = BTreeSet::new();
        while let Some(id) = geometry_roots.pop() {
            if check() {
                return Err(TimelineError::Cancelled.into());
            }
            if geometry_owners.insert((page.slide.clone(), id)) {
                geometry_roots.extend(children.get(&id).into_iter().flatten().copied());
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
            geometry_owners,
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
    pub fn prepare_resize(
        &mut self,
        package: &dyn PackageRead,
        viewport: mo_raster::RasterViewport,
        decoder: &mut dyn mo_image::ImageDecoder,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceViewportUpdate<'_>, SourcePlaybackError> {
        let page = self.page.prepare_resize(
            package,
            viewport.clone(),
            &self.timing.resource_visibility,
            DecodePolicy::Retained(&self.timing.geometry_owners),
            decoder,
            check,
        )?;
        Ok(SourceViewportUpdate {
            page,
            current: &mut self.timing.page.viewport,
            viewport,
        })
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
                decode_policy: DecodePolicy::Viewport,
            },
            decoder,
            text,
            options,
            check,
        )
    }
}

/// Privately admitted resources borrowed from their exact owner. Dropping this
/// candidate aborts; commit is infallible and cannot target another session.
pub struct SourceViewportUpdate<'a> {
    page: ResourceViewportUpdate<'a>,
    current: &'a mut mo_raster::RasterViewport,
    viewport: mo_raster::RasterViewport,
}
impl SourceViewportUpdate<'_> {
    pub fn preparation(&self) -> &ResourcePreparationInfo {
        self.page.info()
    }
    pub fn commit(self) {
        self.page.commit();
        *self.current = self.viewport;
    }
}
