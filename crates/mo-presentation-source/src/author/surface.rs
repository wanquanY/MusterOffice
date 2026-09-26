use super::*;
use crate::source::{drawingml::ColorSlot, effects::*, fill::*};
use mo_common::{Emu, ObjectId};
use mo_presentation_model::*;

pub(super) fn build(
    document: &Document,
    defaults: &ExportDefaults,
    bindings: &NativeBindings,
    index: &mut SourceIndex,
    images: &mut BTreeMap<(String, String), ResourceId>,
    ord: &mut Ordinals,
) -> Result<(), PptxError> {
    for master in &bindings.masters {
        let model = master.id.as_ref().map(|id| &document.masters[id]);
        let background = model.map_or(
            Inherited::Value(Fill::Solid {
                color: Color::Srgb {
                    rgba: defaults.page_background,
                },
            }),
            |m| m.background.clone(),
        );
        let mut surface = empty(SurfaceKind::Master, index, ord)?;
        surface.name = Some(
            master
                .id
                .as_ref()
                .map_or("MusterOffice default", |id| id.as_str())
                .into(),
        );
        surface.links.theme = Some(SourcePartRef {
            part: master.theme.to_string(),
            sha256: index.source_sha256.clone(),
        });
        surface.background = background_decl(&background, ord)?;
        objects(
            document,
            bindings,
            &master.part.to_string(),
            model.map_or(&[], |m| m.objects.as_slice()),
            &mut surface,
            images,
            ord,
        )?;
        let mut text = text::TextBuilder::new(ord);
        text.master(model.map(|m| &m.default_text), document)?;
        surface.text.roots.extend(text.catalog.roots);
        surface.text.nodes.extend(text.catalog.nodes);
        index.surfaces.insert(master.part.to_string(), surface);
    }
    for layout in &bindings.layouts {
        let model = layout.id.as_ref().map(|id| &document.layouts[id]);
        if model.is_some_and(|l| l.default_text != CharacterStyle::default()) {
            return Err(PptxError::Unsupported(
                "layout text defaults require native placeholder bindings".into(),
            ));
        }
        let mut surface = empty(SurfaceKind::Layout, index, ord)?;
        surface.name = Some(model.map_or("Blank", |l| &l.name).into());
        surface.links.master = Some(layout.master.to_string());
        if let Some(model) = model {
            surface.background = background_decl(&model.background, ord)?;
        }
        objects(
            document,
            bindings,
            &layout.part.to_string(),
            model.map_or(&[], |l| l.objects.as_slice()),
            &mut surface,
            images,
            ord,
        )?;
        index.surfaces.insert(layout.part.to_string(), surface);
    }
    for (i, id) in document.slide_order.iter().enumerate() {
        let model = &document.slides[id];
        let part = format!("/ppt/slides/slide{}.xml", i + 1);
        let mut surface = empty(SurfaceKind::Slide, index, ord)?;
        surface.name = Some(model.name.clone());
        surface.hidden = model.hidden;
        surface.links.layout = Some(
            model
                .layout
                .as_ref()
                .map_or(&bindings.layouts[0], |id| {
                    bindings
                        .layouts
                        .iter()
                        .find(|l| l.id.as_ref() == Some(id))
                        .expect("validated layout")
                })
                .part
                .to_string(),
        );
        surface.background = background_decl(&model.background, ord)?;
        objects(
            document,
            bindings,
            &part,
            &model.objects,
            &mut surface,
            images,
            ord,
        )?;
        index.surfaces.insert(part.clone(), surface);
        index.slides.push(SourceSlide {
            native_id: 256 + i as u32,
            part,
        });
    }
    Ok(())
}
fn empty(
    kind: SurfaceKind,
    index: &SourceIndex,
    ord: &mut Ordinals,
) -> Result<SourceSurface, PptxError> {
    let color_mapping = Some(if kind == SurfaceKind::Master {
        use ColorSlot::*;
        SourceColorMapping::Explicit {
            source_ordinal: ord.next()?,
            mapping: SourceColorMap {
                bg1: Lt1,
                tx1: Dk1,
                bg2: Lt2,
                tx2: Dk2,
                accent1: Accent1,
                accent2: Accent2,
                accent3: Accent3,
                accent4: Accent4,
                accent5: Accent5,
                accent6: Accent6,
                hlink: Hlink,
                fol_hlink: FolHlink,
            },
        }
    } else {
        SourceColorMapping::Master {
            source_ordinal: ord.next()?,
        }
    });
    Ok(SourceSurface {
        text: Default::default(),
        show_master_shapes: None,
        visual_issues: vec![],
        color_mapping,
        resolved_color_mapping: None,
        links: Default::default(),
        effective_theme: Default::default(),
        theme_selection: Default::default(),
        compatibility: Default::default(),
        kind,
        sha256: index.source_sha256.clone(),
        name: None,
        hidden: false,
        root_object_id: 1,
        root_group_transform: None,
        background: None,
        root_group_fill: None,
        root_group_effects: None,
        effect_nodes: BTreeMap::new(),
        objects: vec![],
        text_edit_barriers: vec![],
        notices: vec![],
    })
}
fn background_decl(
    value: &Inherited<Fill>,
    ord: &mut Ordinals,
) -> Result<Option<SourceBackground>, PptxError> {
    let Inherited::Value(value) = value else {
        return Ok(None);
    };
    Ok(Some(SourceBackground {
        source_ordinal: ord.next()?,
        black_white_mode: None,
        definition: SourceBackgroundDefinition::Properties {
            source_ordinal: ord.next()?,
            shade_to_title: None,
            fill: Box::new(paint::fill(value, ord)?),
            effects: Some(empty_effects(ord)?),
            retained_ordinals: vec![],
        },
        retained_ordinals: vec![],
    }))
}
pub(super) fn empty_effects(ord: &mut Ordinals) -> Result<SourceEffectProperties, PptxError> {
    Ok(SourceEffectProperties {
        source_ordinal: ord.next()?,
        definition: SourceEffectPropertiesDefinition::List { nodes: vec![] },
        retained_ordinals: vec![],
    })
}
fn objects(
    document: &Document,
    bindings: &NativeBindings,
    part: &str,
    roots: &[ObjectId],
    surface: &mut SourceSurface,
    images: &mut BTreeMap<(String, String), ResourceId>,
    ord: &mut Ordinals,
) -> Result<(), PptxError> {
    // Iterative preorder avoids a second recursion limit below model validation.
    let mut pending: Vec<_> = roots.iter().rev().map(|id| (id, None)).collect();
    let mut next_image = 2;
    while let Some((id, parent_group)) = pending.pop() {
        cancelled(ord.check)?;
        let model = &document.objects[id];
        let direct = model
            .transform
            .as_ref()
            .ok_or_else(|| value_error("transform", "missing authored transform"))?;
        if model.accessibility.decorative {
            return Err(PptxError::Unsupported(
                "decorative accessibility extension".into(),
            ));
        }
        let native_id = bindings.object_ids[id];
        let mut object = SourceObject {
            hidden: None,
            text_body_ordinal: None,
            visual_issues: vec![],
            native_id,
            name: id.to_string(),
            kind: SourceObjectKind::Shape,
            parent_group,
            placeholder: None,
            transform: Some(transform(direct, None)?),
            line: None,
            line_reference: None,
            geometry: None,
            fill: None,
            fill_reference: None,
            picture_fill: None,
            use_background_fill: None,
            effects: None,
            effect_reference: None,
            resolution: Default::default(),
            paragraphs: vec![],
        };
        if let Inherited::Value(fill) = &model.appearance.fill {
            object.fill = Some(paint::fill(fill, ord)?);
        }
        if let Inherited::Value(line) = &model.appearance.stroke {
            object.line = Some(paint::line(line, ord)?);
        }
        match &model.content {
            ObjectContent::RetainedSource { .. } => {
                return Err(PptxError::Unsupported(
                    "retained content requires a source plan".into(),
                ));
            }
            ObjectContent::Shape { geometry, text } => {
                object.geometry = Some(super::geometry::geometry(geometry, direct.size, ord)?);
                if let Some(body) = text {
                    let mut text = text::TextBuilder::new(ord);
                    let (root, paragraphs) = text.body(native_id, body, document)?;
                    object.text_body_ordinal = Some(root);
                    object.paragraphs = paragraphs;
                    surface.text.roots.extend(text.catalog.roots);
                    surface.text.nodes.extend(text.catalog.nodes);
                }
            }
            ObjectContent::Picture { resource, crop } => {
                object.kind = SourceObjectKind::Picture;
                object.geometry = Some(super::geometry::geometry(
                    &Geometry::Rectangle,
                    direct.size,
                    ord,
                )?);
                let reference = format!("rId{next_image}");
                next_image += 1;
                images.insert((part.into(), reference.clone()), resource.clone());
                object.picture_fill = Some(paint::image(reference, *crop, ord)?);
            }
            ObjectContent::Group { children, viewport } => {
                if model.appearance.stroke != Inherited::Inherit {
                    return Err(PptxError::Unsupported("group stroke semantics".into()));
                }
                object.kind = SourceObjectKind::Group;
                object.transform = Some(transform(direct, Some(*viewport))?);
                pending.extend(children.iter().rev().map(|id| (id, Some(native_id))));
            }
            ObjectContent::Connector { start, end } => {
                connector(document, model, start, false)?;
                connector(document, model, end, true)?;
                object.kind = SourceObjectKind::Connector;
                object.geometry = Some(crate::source::geometry::SourceGeometry {
                    source_ordinal: ord.next()?,
                    definition: crate::source::geometry::SourceGeometryDefinition::Preset {
                        preset: native("line".into())?,
                        adjustments: Some(crate::source::geometry::SourceGeometryList {
                            source_ordinal: ord.next()?,
                            entries: vec![],
                        }),
                    },
                    retained_ordinals: vec![],
                });
            }
        }
        surface.objects.push(object);
    }
    Ok(())
}
fn transform(t: &Transform, viewport: Option<Size>) -> Result<SourceTransform, PptxError> {
    for v in [t.origin.x, t.origin.y, t.size.width, t.size.height] {
        super::geometry::coordinate(v)?;
    }
    if let Some(v) = viewport {
        super::geometry::coordinate(v.width)?;
        super::geometry::coordinate(v.height)?;
    }
    Ok(SourceTransform {
        retained_ordinals: vec![],
        origin: Some(t.origin),
        size: Some(t.size),
        child_origin: viewport.map(|_| Point {
            x: Emu::ZERO,
            y: Emu::ZERO,
        }),
        child_size: viewport,
        rotation: Some(t.normalized_rotation()),
        flip_horizontal: Some(t.flip_horizontal),
        flip_vertical: Some(t.flip_vertical),
    })
}
fn connector(
    document: &Document,
    model: &Object,
    endpoint: &ConnectorEndpoint,
    end: bool,
) -> Result<(), PptxError> {
    match endpoint {
        ConnectorEndpoint::Attached { object, site } => {
            if *site > 3
                || !matches!(
                    document.objects[object].content,
                    ObjectContent::Shape {
                        geometry: Geometry::Rectangle
                            | Geometry::Ellipse
                            | Geometry::RoundRectangle { .. },
                        ..
                    }
                )
            {
                return Err(PptxError::Unsupported(
                    "connector site on custom geometry or non-shape".into(),
                ));
            }
        }
        ConnectorEndpoint::Free { position } => {
            let t = model
                .transform
                .as_ref()
                .ok_or_else(|| value_error("transform", "missing authored transform"))?;
            let expected = if end {
                Point {
                    x: t.origin
                        .x
                        .checked_add(t.size.width)
                        .map_err(|_| value_error("connector", "overflow"))?,
                    y: t.origin
                        .y
                        .checked_add(t.size.height)
                        .map_err(|_| value_error("connector", "overflow"))?,
                }
            } else {
                t.origin
            };
            if *position != expected
                || t.normalized_rotation() != 0
                || t.flip_horizontal
                || t.flip_vertical
            {
                return Err(PptxError::Unsupported(
                    "free connector endpoint normalization".into(),
                ));
            }
        }
    }
    Ok(())
}
