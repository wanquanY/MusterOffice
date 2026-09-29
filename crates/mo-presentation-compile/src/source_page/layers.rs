//! Explicit static page composition; source declarations remain immutable.
use super::*;
use mo_presentation_source::source::{
    PlaceholderKind, SourceObject, SourceObjectKind, SourcePlaceholderMatch, SourceSurface,
    SurfaceKind,
};
use std::collections::{BTreeMap, BTreeSet};

fn surface<'a>(
    index: &'a SourceIndex,
    part: &str,
    kind: SurfaceKind,
) -> Result<&'a SourceSurface, SourcePageError> {
    index
        .surfaces
        .get(part)
        .filter(|s| s.kind == kind)
        .ok_or(SourcePageError::Invalid("surface hierarchy binding"))
}
pub(super) fn select(
    index: &SourceIndex,
    q: &SourcePageRequest,
    text_enabled: bool,
    images_enabled: bool,
    properties: Option<&crate::source_placement::SourceProperties>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<SourcePageLayer>, SourcePageError> {
    if !index.slides.iter().any(|s| s.part == q.slide) {
        return Err(SourcePageError::Invalid(
            "slide is not in presentation order",
        ));
    }
    let slide = surface(index, &q.slide, SurfaceKind::Slide)?;
    let mut chain = vec![];
    if let Some(part) = &slide.links.layout {
        let layout = surface(index, part, SurfaceKind::Layout)?;
        if let Some(part) = &layout.links.master {
            surface(index, part, SurfaceKind::Master)?;
            chain.push((
                part,
                slide.show_master_shapes != Some(false) && layout.show_master_shapes != Some(false),
            ));
        }
        chain.push((part, slide.show_master_shapes != Some(false)));
    }
    chain.push((&q.slide, true));
    // A layout declaration is nearer than the master. Once a declaration is
    // present, omitted attributes use CT_HeaderFooter's enabled default rather
    // than inheriting a disabled attribute from a more distant declaration.
    let header_footer = chain.iter().rev().find_map(|(part, _)| {
        index
            .surfaces
            .get(part.as_str())
            .and_then(|s| s.header_footer.as_ref())
    });
    let mut layers = vec![];
    let mut count = 0usize;
    for (part, visible) in chain {
        cancel(check)?;
        let s = &index.surfaces[part];
        let location = SourcePageLocation {
            part: part.clone(),
            object: None,
        };
        let mut layer = SourcePageLayer {
            part: part.clone(),
            kind: s.kind,
            visible,
            objects: vec![],
            hidden_objects: vec![],
            template_placeholders: vec![],
        };
        if let Some(issue) = s.visual_issues.first() {
            return Err(mapping(
                &location,
                SourcePageIssue::Visual {
                    issue: issue.clone(),
                },
            ));
        }
        if visible {
            if let Some(e) = &s.root_group_effects
                && !e.is_explicitly_empty_list()
            {
                return Err(mapping(
                    &location,
                    SourcePageIssue::Effects {
                        source_ordinal: e.source_ordinal,
                    },
                ));
            }
            let mut groups = BTreeMap::new();
            let mut ids = BTreeSet::new();
            for o in &s.objects {
                cancel(check)?;
                count += 1;
                if count > 8192 {
                    return Err(mo_raster::RasterError::Limit("source page objects").into());
                }
                if !ids.insert(o.native_id) {
                    return Err(SourcePageError::Invalid("duplicate source object"));
                }
                let parent_hidden = match o.parent_group {
                    None => false,
                    Some(id) => *groups.get(&id).ok_or(SourcePageError::Invalid(
                        "source parent is not an earlier group",
                    ))?,
                };
                let own_hidden = properties
                    .and_then(|p| p.get(&(part.clone(), o.native_id)))
                    .and_then(|p| p.visibility)
                    .map_or(o.hidden == Some(true), |v| {
                        v == mo_timeline::Visibility::Hidden
                    });
                let hidden = parent_hidden || own_hidden;
                if o.kind == SourceObjectKind::Group {
                    groups.insert(o.native_id, hidden);
                }
                if hidden {
                    layer.hidden_objects.push(o.native_id);
                    continue;
                }
                let at = SourcePageLocation {
                    part: part.clone(),
                    object: Some(o.native_id),
                };
                if s.kind != SurfaceKind::Slide
                    && let Some(ph) = &o.placeholder
                {
                    if header_footer.is_some_and(|hf| hf.disables(ph.effective_kind())) {
                        layer.hidden_objects.push(o.native_id);
                        continue;
                    }
                    if matches!(
                        ph.effective_kind(),
                        PlaceholderKind::Date
                            | PlaceholderKind::Footer
                            | PlaceholderKind::Header
                            | PlaceholderKind::SlideNumber
                            | PlaceholderKind::SlideImage
                    ) {
                        return Err(mapping(&at, SourcePageIssue::SpecialPlaceholder {}));
                    }
                    // Template prompt shapes supply inheritance, not extra ink.
                    if o.kind == SourceObjectKind::Group {
                        return Err(mapping(
                            &at,
                            SourcePageIssue::Object {
                                native_kind: o.kind,
                            },
                        ));
                    }
                    layer.template_placeholders.push(o.native_id);
                    continue;
                }
                audit(index, part, o, text_enabled, images_enabled, check)?;
                if o.kind != SourceObjectKind::Group {
                    layer.objects.push(o.native_id);
                }
            }
        }
        layers.push(layer);
    }
    Ok(layers)
}
fn audit(
    index: &SourceIndex,
    part: &str,
    object: &SourceObject,
    text_enabled: bool,
    images_enabled: bool,
    check: &dyn Fn() -> bool,
) -> Result<(), SourcePageError> {
    let location = SourcePageLocation {
        part: part.into(),
        object: Some(object.native_id),
    };
    if let Some(source_ordinal) = object.text_body_ordinal.or_else(|| {
        object.table.as_ref().and_then(|t| {
            t.rows
                .iter()
                .flat_map(|r| &r.cells)
                .find_map(|c| c.text_body_ordinal)
        })
    }) && !text_enabled
    {
        if images_enabled {
            return Err(SourcePageError::TextContextRequired {
                location,
                source_ordinal,
            });
        }
        return Err(mapping(&location, SourcePageIssue::Text { source_ordinal }));
    }
    if !(matches!(
        object.kind,
        SourceObjectKind::Shape | SourceObjectKind::Connector | SourceObjectKind::Group
    ) || images_enabled && object.kind == SourceObjectKind::Picture
        || object.kind == SourceObjectKind::GraphicFrame && object.table.is_some())
    {
        return Err(mapping(
            &location,
            SourcePageIssue::Object {
                native_kind: object.kind,
            },
        ));
    }
    let mut part = part;
    let mut object = object;
    let mut effect_boundary = false;
    // Only three physical surfaces are legal. This also bounds malicious index
    // cycles without a recursive traversal or searching arbitrary parts.
    for _ in 0..3 {
        cancel(check)?;
        let at = SourcePageLocation {
            part: part.into(),
            object: Some(object.native_id),
        };
        if let Some(issue) = object.visual_issues.first() {
            return Err(mapping(
                &at,
                SourcePageIssue::Visual {
                    issue: issue.clone(),
                },
            ));
        }
        if !effect_boundary {
            if let Some(e) = &object.effects {
                if !e.is_explicitly_empty_list() {
                    return Err(mapping(
                        &at,
                        SourcePageIssue::Effects {
                            source_ordinal: e.source_ordinal,
                        },
                    ));
                }
                effect_boundary = true;
            } else if let Some(r) = &object.effect_reference {
                // The full effect style cascade has not been connected. Even
                // idx=0 is diagnosed instead of guessing an inheritance rule.
                return Err(mapping(
                    &at,
                    SourcePageIssue::Effects {
                        source_ordinal: r.source_ordinal,
                    },
                ));
            }
        }
        match &object.resolution.placeholder_match {
            SourcePlaceholderMatch::Matched { target, .. } => {
                let s = &index.surfaces[part];
                let parent = match s.kind {
                    SurfaceKind::Slide => &s.links.layout,
                    SurfaceKind::Layout => &s.links.master,
                    SurfaceKind::Master => {
                        return Err(SourcePageError::Invalid("master placeholder has parent"));
                    }
                };
                if parent.as_deref() != Some(target.part.as_str()) {
                    return Err(SourcePageError::Invalid(
                        "placeholder is not on linked parent",
                    ));
                }
                part = &target.part;
                let s = index
                    .surfaces
                    .get(part)
                    .ok_or(SourcePageError::Invalid("placeholder surface"))?;
                object = s
                    .objects
                    .iter()
                    .find(|o| o.native_id == target.native_id)
                    .ok_or(SourcePageError::Invalid("placeholder object"))?;
            }
            SourcePlaceholderMatch::NotPlaceholder
            | SourcePlaceholderMatch::Master
            | SourcePlaceholderMatch::Detached => return Ok(()),
            matching => {
                return Err(mapping(
                    &at,
                    SourcePageIssue::Placeholder {
                        matching: matching.clone(),
                    },
                ));
            }
        }
    }
    Err(SourcePageError::Invalid("placeholder depth"))
}
