//! Immutable two-phase frame preparation. The host resolves exact requests
//! asynchronously, then core validates all replies before emitting GPU work.
use crate::{GpuImage, GpuProgram, ProgramLimits, compile_gpu_program};
use mo_common::Digest;
use mo_time::ExactInterval;
use mo_video_compile::{MediaBinding, VideoFramePlan, VisualNode, VisualSource};
use mo_video_model::TimeSample;
use mo_video_render::{
    FrameRenderOptions, RenderError, ResolvedMedia, ResolvedVisual, Result, SourceGeometry,
    VisualFrame, VisualResources, validate_frame_plan,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualRequest {
    /// Opaque ID local to this candidate, used by the resulting GPU program.
    pub image_id: u32,
    /// Content identity of full binding + exact sample, including before/after.
    /// This is not an authorization token or a storage locator.
    pub key: Digest,
    pub binding: MediaBinding,
    pub sample: TimeSample,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisplayGeometry {
    pub sample_aspect_ratio: [u32; 2],
    /// Container row-vector matrix, not a CSS matrix or an already displayed frame.
    pub matrix: Option<[f64; 9]>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageDescription {
    pub width: u32,
    pub height: u32,
    pub has_hdr: bool,
    pub display: DisplayGeometry,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum VisualDescription {
    VideoFrame {
        interval: ExactInterval,
        image: ImageDescription,
    },
    VideoGap {
        interval: ExactInterval,
    },
    Image {
        image: ImageDescription,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualReply {
    pub request: Digest,
    pub binding: MediaBinding,
    pub visual: VisualDescription,
}
pub struct PreparedGpuFrame {
    plan: VideoFramePlan,
    options: FrameRenderOptions,
    limits: ProgramLimits,
    requests: Vec<VisualRequest>,
    by_key: BTreeMap<Digest, usize>,
}
fn key(binding: &MediaBinding, sample: &TimeSample) -> Result<Digest> {
    sample.time.check_budget(4096)?;
    mo_common::digest("musteroffice.video.visual-request/1", &(binding, sample))
        .map_err(|_| RenderError::Invalid("visual request identity"))
}
impl PreparedGpuFrame {
    pub fn new(
        plan: VideoFramePlan,
        options: FrameRenderOptions,
        limits: ProgramLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        validate_frame_plan(&plan, options, check)?;
        limits.validate()?;
        let mut requests = Vec::new();
        let mut by_key = BTreeMap::new();
        collect(
            &plan.layers,
            &mut requests,
            &mut by_key,
            limits.max_images,
            check,
        )?;
        Ok(Self {
            plan,
            options,
            limits,
            requests,
            by_key,
        })
    }
    pub fn plan(&self) -> &VideoFramePlan {
        &self.plan
    }
    pub fn requests(&self) -> &[VisualRequest] {
        &self.requests
    }
    /// A complete immutable reply set is required. Errors never consume the
    /// candidate, so retries cannot retain a partially resolved source table.
    pub fn compile(
        &self,
        replies: Vec<VisualReply>,
        check: &dyn Fn() -> bool,
    ) -> Result<GpuProgram> {
        if replies.len() != self.requests.len() {
            return Err(RenderError::Invalid("visual reply count"));
        }
        let mut table = BTreeMap::new();
        for reply in replies {
            cancelled(check)?;
            let Some(index) = self.by_key.get(&reply.request) else {
                return Err(RenderError::Invalid("unexpected visual reply"));
            };
            if reply.binding != self.requests[*index].binding {
                return Err(RenderError::Invalid("visual reply binding"));
            }
            let id = self.requests[*index].image_id;
            let frame = |image: ImageDescription| -> Result<VisualFrame<GpuImage>> {
                Ok(VisualFrame {
                    image: GpuImage::new(id, image.width, image.height, image.has_hdr)?,
                    geometry: SourceGeometry::from_display_metadata(
                        (
                            image.display.sample_aspect_ratio[0],
                            image.display.sample_aspect_ratio[1],
                        ),
                        image.display.matrix,
                    )?,
                })
            };
            let visual = match reply.visual {
                VisualDescription::VideoFrame { interval, image } => ResolvedVisual::VideoFrame {
                    interval,
                    frame: frame(image)?,
                },
                VisualDescription::VideoGap { interval } => ResolvedVisual::VideoGap { interval },
                VisualDescription::Image { image } => ResolvedVisual::Image {
                    frame: frame(image)?,
                },
            };
            if table
                .insert(
                    reply.request,
                    ResolvedMedia {
                        binding: reply.binding,
                        visual,
                    },
                )
                .is_some()
            {
                return Err(RenderError::Invalid("duplicate visual reply"));
            }
        }
        compile_gpu_program(
            &self.plan,
            &mut Replies(table),
            self.options,
            self.limits,
            check,
        )
    }
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(RenderError::Cancelled)
    } else {
        Ok(())
    }
}
fn collect(
    nodes: &[VisualNode],
    requests: &mut Vec<VisualRequest>,
    by_key: &mut BTreeMap<Digest, usize>,
    maximum: usize,
    check: &dyn Fn() -> bool,
) -> Result<()> {
    // Shared preflight has already bounded depth/nodes/effects before traversal.
    for node in nodes {
        cancelled(check)?;
        match &node.source {
            VisualSource::Media { binding, mapping } => {
                let key = key(binding, &mapping.sample)?;
                if by_key.contains_key(&key) {
                    continue;
                }
                if requests.len() >= maximum {
                    return Err(RenderError::Budget);
                }
                by_key.insert(key.clone(), requests.len());
                requests.push(VisualRequest {
                    image_id: requests.len() as u32 + 1,
                    key,
                    binding: binding.clone(),
                    sample: mapping.sample.clone(),
                });
            }
            VisualSource::Group { layers, .. } => {
                collect(layers, requests, by_key, maximum, check)?
            }
            _ => {}
        }
    }
    Ok(())
}
struct Replies(BTreeMap<Digest, ResolvedMedia<GpuImage>>);
impl VisualResources<GpuImage> for Replies {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia<GpuImage>> {
        cancelled(check)?;
        self.0
            .get(&key(binding, sample)?)
            .cloned()
            .ok_or(RenderError::Invalid("missing visual reply"))
    }
}
