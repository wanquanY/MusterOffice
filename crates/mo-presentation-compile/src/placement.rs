use crate::{
    CompileError, ObjectPlacement, PROFILE, PagePlacementRequest, PagePlacements, PlacementSurface,
    cancel,
    placement_core::{Engine, Frame, State},
};
use crate::{angle::Angle, sampled_properties::SampledProperties};
use mo_common::{Digest, Emu, ObjectId};
use mo_presentation_model::{self as model, ContainerId, ObjectContent, ValidationLimits};
use std::collections::BTreeMap;
pub fn page_placements(
    request: &PagePlacementRequest,
    check: &dyn Fn() -> bool,
) -> Result<PagePlacements, CompileError> {
    cancel(check)?;
    let d = &request.document;
    let report = model::validate(
        d,
        ValidationLimits {
            max_objects: 8192,
            ..ValidationLimits::default()
        },
    );
    cancel(check)?;
    if !report.is_valid() {
        return Err(CompileError::Document(report));
    }
    let document_sha256 = d
        .semantic_digest()
        .map_err(|_| CompileError::Invalid("document identity"))?;
    place_validated(request, &document_sha256, &BTreeMap::new(), check)
}
/// Only used with the immutable document already admitted by page_placements.
pub(crate) fn place_validated(
    request: &PagePlacementRequest,
    document_sha256: &Digest,
    transforms: &BTreeMap<ObjectId, SampledProperties>,
    check: &dyn Fn() -> bool,
) -> Result<PagePlacements, CompileError> {
    cancel(check)?;
    let d = &request.document;
    if d.source_bindings.is_some() {
        return Err(CompileError::Invalid(
            "retained document requires source plan placement",
        ));
    }
    let slide = d
        .slides
        .get(&request.slide)
        .ok_or(CompileError::Invalid("slide reference"))?;
    let mut roots = vec![];
    if let Some(id) = &slide.layout {
        let layout = &d.layouts[id];
        let master = &d.masters[&layout.master];
        roots.push((ContainerId::Master(master.id.clone()), &master.objects));
        roots.push((ContainerId::Layout(layout.id.clone()), &layout.objects));
    }
    roots.push((ContainerId::Slide(slide.id.clone()), &slide.objects));
    let mut engine = Engine::new();
    let mut surfaces = vec![];
    for (container, ids) in roots {
        cancel(check)?;
        let mut objects = vec![];
        let mut pending: Vec<(&ObjectId, State, u32)> = ids
            .iter()
            .rev()
            .map(|id| (id, State::identity(), 0))
            .collect();
        while let Some((id, parent, depth)) = pending.pop() {
            cancel(check)?;
            // A hidden group suppresses its subtree. Child visibility cannot
            // punch through an invisible ancestor; geometry is unchanged.
            if transforms.get(id).and_then(|v| v.visibility)
                == Some(mo_timeline::Visibility::Hidden)
            {
                continue;
            }
            let o = &d.objects[id];
            let t = o
                .transform
                .as_ref()
                .ok_or(CompileError::Invalid("missing author transform"))?;
            let source_size = match &o.content {
                ObjectContent::Group { viewport, .. }
                | ObjectContent::Shape {
                    geometry: model::Geometry::Path { viewport, .. },
                    ..
                } => *viewport,
                _ => t.size,
            };
            let placed = engine.place(
                &Frame {
                    origin: t.origin,
                    target_size: t.size,
                    source_origin: model::Point {
                        x: Emu::new(0),
                        y: Emu::new(0),
                    },
                    source_size,
                    rotation: transforms
                        .get(id)
                        .and_then(|v| v.rotation.as_ref())
                        .map(|v| v.resolve(Angle::integer(i64::from(t.normalized_rotation()))))
                        .transpose()?
                        .unwrap_or_else(|| Angle::integer(i64::from(t.normalized_rotation()))),
                    scale: transforms.get(id).and_then(|v| v.scale.clone()),
                    motion: transforms.get(id).and_then(|v| v.motion_emu(d.page_size)),
                    flips: [t.flip_horizontal, t.flip_vertical],
                },
                &parent,
                matches!(o.content, ObjectContent::Group { .. }),
                check,
            )?;
            objects.push(ObjectPlacement {
                object: id.clone(),
                parent: o.parent.clone(),
                depth,
                source_size,
                anchor: placed.anchor,
                affine: placed.affine,
                uncertainty: placed.uncertainty,
            });
            if let ObjectContent::Group { children, .. } = &o.content {
                let state = placed
                    .children
                    .expect("group requested child coordinate state");
                for child in children.iter().rev() {
                    cancel(check)?;
                    pending.push((child, state.clone(), depth + 1));
                }
            }
        }
        surfaces.push(PlacementSurface { container, objects });
    }
    cancel(check)?;
    Ok(PagePlacements {
        profile: PROFILE.into(),
        document_sha256: document_sha256.clone(),
        slide: slide.id.clone(),
        hidden: slide.hidden,
        page_size: d.page_size,
        surfaces,
        unique_angles: engine.unique_angles(),
    })
}
