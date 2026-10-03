use super::*;
use mo_geometry::{Point, Rect};
use mo_presentation_source::source::SourceObjectRef;
use mo_raster::picking::{DevicePickQuery, DrawHitKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePickQuery {
    /// Device pixels of this exact prepared viewport, including pointer tolerance.
    pub device: DevicePickQuery,
    /// 1..=256 unique objects, in reverse page paint order.
    pub max_hits: u32,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageObjectHit {
    pub object: SourceObjectRef,
    pub binding: u32,
    pub kind: DrawHitKind,
    /// Original page draw; None denotes only a semantic text-frame interior.
    pub instance: Option<u32>,
    /// Glyph, decoration or semantic interior of a native text frame/table cell.
    pub text_frame: Option<u32>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagePickResult {
    pub hits: Vec<PageObjectHit>,
    pub truncated: bool,
}
#[derive(Clone, Copy)]
struct Target {
    object: u32,
    binding: u32,
    text_frame: Option<u32>,
}
pub(super) struct PickIndex {
    objects: Vec<SourceObjectRef>,
    bindings: Vec<Option<u32>>,
    draws: Vec<Option<Target>>,
}
impl PickIndex {
    pub(super) fn new(
        page: &SourceResourcePagePlan,
        draws: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let invalid = || SourcePageError::Invalid("editor picking provenance");
        let mut index = Self {
            objects: vec![],
            bindings: vec![],
            draws: vec![],
        };
        let mut ids = BTreeMap::new();
        for binding in &page.page.bindings {
            if check() {
                return Err(mo_raster::RasterError::Cancelled.into());
            }
            let object = binding.location.object.map(|native_id| {
                let key = (binding.location.part.clone(), native_id);
                *ids.entry(key).or_insert_with(|| {
                    let id = index.objects.len() as u32;
                    index.objects.push(SourceObjectRef {
                        part: binding.location.part.clone(),
                        native_id,
                    });
                    id
                })
            });
            index.bindings.push(object);
        }
        if draws as usize != page.page.paint_sources.len() {
            return Err(invalid());
        }
        for (i, source) in page.page.paint_sources.iter().enumerate() {
            if check() {
                return Err(mo_raster::RasterError::Cancelled.into());
            }
            if source.instance as usize != i {
                return Err(invalid());
            }
            let object = *index
                .bindings
                .get(source.binding as usize)
                .ok_or_else(invalid)?;
            index.draws.push(object.map(|object| Target {
                object,
                binding: source.binding,
                text_frame: None,
            }));
        }
        let text = page.text.as_ref().ok_or_else(invalid)?;
        for (instance, frame) in text
            .text_sources
            .iter()
            .map(|s| (s.instance, s.text_binding))
            .chain(
                text.decoration_sources
                    .iter()
                    .map(|s| (s.instance, s.text_binding)),
            )
        {
            if check() {
                return Err(mo_raster::RasterError::Cancelled.into());
            }
            let t = text.texts.get(frame as usize).ok_or_else(invalid)?;
            let target = index
                .draws
                .get_mut(instance as usize)
                .and_then(Option::as_mut)
                .ok_or_else(invalid)?;
            if target.binding != t.binding || target.text_frame.is_some() {
                return Err(invalid());
            }
            target.text_frame = Some(frame);
        }
        Ok(index)
    }
}
#[derive(Clone, Copy)]
struct Candidate {
    target: Target,
    kind: DrawHitKind,
    instance: Option<u32>,
}
fn contains(rect: Rect, point: Point) -> bool {
    rect.min.x < rect.max.x
        && rect.min.y < rect.max.y
        && point.x >= rect.min.x
        && point.x <= rect.max.x
        && point.y >= rect.min.y
        && point.y <= rect.max.y
}
impl SourceEditorPage {
    pub fn objects(&self) -> &[SourceObjectRef] {
        &self.pick_index.objects
    }

    /// Paint masks use the exact device batch captured during prepare. Blank
    /// text interiors reuse computed native frames and those same clip masks.
    /// No layout, image decoding, shaping or rasterization runs during picking.
    pub fn pick(
        &self,
        queries: &[PagePickQuery],
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<PagePickResult>, SourcePageError> {
        if check() {
            return Err(mo_raster::RasterError::Cancelled.into());
        }
        if queries.len() > 64 {
            return Err(mo_raster::RasterError::Limit("page pick queries").into());
        }
        if queries.iter().any(|q| !(1..=256).contains(&q.max_hits)) {
            return Err(SourcePageError::Invalid("page pick result limit"));
        }
        let text = self.page.text.as_ref().expect("editor text context");
        let work = queries.len().checked_mul(
            self.pick_index.draws.len() + text.texts.len() + self.pick_index.objects.len(),
        );
        if work.is_none_or(|w| w > 8_388_608) {
            return Err(mo_raster::RasterError::Limit("page pick mapping work").into());
        }
        let device: Vec<_> = queries.iter().map(|q| q.device).collect();
        let sampled = self.picking.query(&device, backend, check)?;
        let mut results = Vec::with_capacity(queries.len());
        for (query, masks) in queries.iter().zip(sampled) {
            let mut candidates = vec![None::<Candidate>; self.pick_index.objects.len()];
            for hit in masks.hits() {
                if check() {
                    return Err(mo_raster::RasterError::Cancelled.into());
                }
                let Some(target) = self.pick_index.draws[hit.draw as usize] else {
                    continue;
                };
                let current = &mut candidates[target.object as usize];
                if current
                    .is_none_or(|c| c.kind == DrawHitKind::Nearby && hit.kind == DrawHitKind::Exact)
                {
                    *current = Some(Candidate {
                        target,
                        kind: hit.kind,
                        instance: Some(hit.draw),
                    });
                }
            }
            let point =
                transform::page_point(&self.page.page.raster.viewport, masks.sampled_point())?;
            for (frame, t) in text.texts.iter().enumerate().rev() {
                if check() {
                    return Err(mo_raster::RasterError::Cancelled.into());
                }
                if !masks.contains_clip(self.interaction.clips[frame]) {
                    continue;
                }
                let transform = transform::Transform {
                    object: &self.page.page.bindings[t.binding as usize],
                    viewport: &self.page.page.raster.viewport,
                    uncertainty: t.local_coordinate_error_bound,
                };
                let Some(local) = transform.inverse(point)? else {
                    continue;
                };
                if !contains(t.frame.region.outer, local) {
                    continue;
                }
                let Some(object) = self.pick_index.bindings[t.binding as usize] else {
                    continue;
                };
                let current = &mut candidates[object as usize];
                match current {
                    Some(c) if c.kind == DrawHitKind::Exact => {
                        if c.target.text_frame.is_none() {
                            c.target.text_frame = Some(frame as u32);
                        }
                    }
                    _ => {
                        *current = Some(Candidate {
                            target: Target {
                                object,
                                binding: t.binding,
                                text_frame: Some(frame as u32),
                            },
                            kind: DrawHitKind::Exact,
                            instance: None,
                        })
                    }
                }
            }
            let mut hits = Vec::new();
            let mut truncated = false;
            for c in candidates.into_iter().rev().flatten() {
                if check() {
                    return Err(mo_raster::RasterError::Cancelled.into());
                }
                if hits.len() == query.max_hits as usize {
                    truncated = true;
                    break;
                }
                hits.push(PageObjectHit {
                    object: self.pick_index.objects[c.target.object as usize].clone(),
                    binding: c.target.binding,
                    kind: c.kind,
                    instance: c.instance,
                    text_frame: c.target.text_frame,
                });
            }
            results.push(PagePickResult { hits, truncated });
        }
        if check() {
            return Err(mo_raster::RasterError::Cancelled.into());
        }
        Ok(results)
    }
}
