use super::*;
use mo_common::{LayoutId, MasterId, ObjectId, ParagraphId, RunId, SlideId, ThemeId};
use mo_presentation_model::*;

pub(super) fn project(
    bound: &BoundIndex,
    id: DocumentId,
    resource: ResourceId,
    title: String,
    check: &dyn Fn() -> bool,
) -> Result<Document, PptxError> {
    project_scoped(
        bound,
        id,
        resource,
        Projection {
            identity_scope: None,
            profile: SourceBindingProfile::PresentationmlRetainedFieldsV5,
            title,
        },
        check,
    )
}

pub(super) struct Projection {
    pub identity_scope: Option<DocumentId>,
    pub profile: SourceBindingProfile,
    pub title: String,
}

pub(super) fn project_scoped(
    bound: &BoundIndex,
    id: DocumentId,
    resource: ResourceId,
    projection: Projection,
    check: &dyn Fn() -> bool,
) -> Result<Document, PptxError> {
    let Projection {
        identity_scope,
        profile,
        title,
    } = projection;
    let index = &bound.index;
    if index.main_content_type
        != "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
    {
        return Err(PptxError::Unsupported(
            "revisioned import currently requires a PPTX presentation package".into(),
        ));
    }
    let seed = (identity_scope.as_ref().unwrap_or(&id), &index.source_sha256);
    let stable = |kind: &str, address: &str| -> String {
        digest("musteroffice.source-domain-id/1", &(&seed, kind, address))
            .expect("finite native identity")
            .to_string()
    };
    let slide_id = |part: &str| SlideId::new(stable("slide", part)).expect("digest ID");
    let master_id = |part: &str| MasterId::new(stable("master", part)).expect("digest ID");
    let layout_id = |part: &str| LayoutId::new(stable("layout", part)).expect("digest ID");
    let theme_id = |part: &str| ThemeId::new(stable("theme", part)).expect("digest ID");
    let object_id = |part: &str, id| {
        ObjectId::new(stable("object", &format!("{part}#{id}"))).expect("digest ID")
    };
    let mut d = Document::empty(
        id.clone(),
        index.page_size.ok_or_else(|| {
            PptxError::Unsupported("source document without explicit page size".into())
        })?,
    );
    d.title = title;
    d.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            kind: ResourceKind::SourcePackage,
            sha256: index.source_sha256.clone(),
            media_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                .into(),
        },
    );
    let mut bindings = SourceBindings {
        profile,
        identity_scope: identity_scope.clone(),
        resource,
        slides: BTreeMap::new(),
        masters: BTreeMap::new(),
        layouts: BTreeMap::new(),
        themes: BTreeMap::new(),
        objects: BTreeMap::new(),
    };
    // These records retain the graph, not a flattened approximation of native
    // styles. Their unpromoted properties are owned by the bound source part.
    for part in index.themes.keys() {
        cancelled(check)?;
        let id = theme_id(part);
        bindings.themes.insert(id.clone(), part.clone());
        d.themes.insert(
            id.clone(),
            Theme {
                id,
                name: String::new(),
                colors: BTreeMap::new(),
                default_text: Default::default(),
            },
        );
    }
    for (part, surface) in &index.surfaces {
        cancelled(check)?;
        let roots: Vec<_> = surface
            .objects
            .iter()
            .filter(|o| o.parent_group.is_none())
            .map(|o| object_id(part, o.native_id))
            .collect();
        let owner = match surface.kind {
            SurfaceKind::Slide => {
                let id = slide_id(part);
                bindings.slides.insert(id.clone(), part.clone());
                d.slides.insert(
                    id.clone(),
                    Slide {
                        id: id.clone(),
                        name: surface.name.clone().unwrap_or_default(),
                        layout: surface.links.layout.as_deref().map(layout_id),
                        objects: roots,
                        background: Inherited::Inherit,
                        hidden: surface.hidden,
                    },
                );
                ContainerId::Slide(id)
            }
            SurfaceKind::Master => {
                let id = master_id(part);
                let theme = surface.links.theme.as_ref().ok_or_else(|| {
                    PptxError::Unsupported("source master without theme relationship".into())
                })?;
                bindings.masters.insert(id.clone(), part.clone());
                d.masters.insert(
                    id.clone(),
                    Master {
                        id: id.clone(),
                        theme: theme_id(&theme.part),
                        objects: roots,
                        background: Inherited::Inherit,
                        default_text: Default::default(),
                    },
                );
                ContainerId::Master(id)
            }
            SurfaceKind::Layout => {
                let id = layout_id(part);
                let master = surface.links.master.as_ref().ok_or_else(|| {
                    PptxError::Unsupported("source layout without master relationship".into())
                })?;
                bindings.layouts.insert(id.clone(), part.clone());
                d.layouts.insert(
                    id.clone(),
                    Layout {
                        id: id.clone(),
                        master: master_id(master),
                        name: surface.name.clone().unwrap_or_default(),
                        objects: roots,
                        background: Inherited::Inherit,
                        default_text: Default::default(),
                    },
                );
                ContainerId::Layout(id)
            }
        };
        let mut children: BTreeMap<u32, Vec<ObjectId>> = BTreeMap::new();
        for object in &surface.objects {
            if let Some(parent) = object.parent_group {
                children
                    .entry(parent)
                    .or_default()
                    .push(object_id(part, object.native_id));
            }
        }
        for object in &surface.objects {
            cancelled(check)?;
            let id = object_id(part, object.native_id);
            let transform = object.transform.as_ref().and_then(|t| {
                Some(Transform {
                    origin: t.origin?,
                    size: t.size?,
                    rotation: t.rotation.unwrap_or(0),
                    flip_horizontal: t.flip_horizontal.unwrap_or(false),
                    flip_vertical: t.flip_vertical.unwrap_or(false),
                })
            });
            let transform_constraint = if index.contains_signatures {
                Some(NativeEditConstraint::RetainedReferences)
            } else if transform.is_none() {
                Some(NativeEditConstraint::MissingDirectTransform)
            } else if object
                .transform
                .as_ref()
                .is_some_and(|t| !t.retained_ordinals.is_empty())
            {
                Some(NativeEditConstraint::RetainedTransform)
            } else if bound
                .transforms
                .get(part)
                .and_then(|b| b.get(&object.native_id))
                .is_none_or(|b| b.in_alternate)
            {
                Some(NativeEditConstraint::CompatibilityBranch)
            } else {
                None
            };
            let mut native = NativeObjectBinding {
                part: part.clone(),
                native_id: object.native_id,
                transform_constraint,
                runs: BTreeMap::new(),
            };
            let mut paragraphs = Vec::new();
            let projected_paragraphs = if object.table.is_some()
                && !matches!(
                    profile,
                    SourceBindingProfile::PresentationmlRetainedFieldsV3
                        | SourceBindingProfile::PresentationmlRetainedFieldsV4
                        | SourceBindingProfile::PresentationmlRetainedFieldsV5
                ) {
                &[][..]
            } else {
                object.paragraphs.as_slice()
            };
            for (pi, runs) in projected_paragraphs.iter().enumerate() {
                let mut paragraph = RetainedParagraph {
                    id: ParagraphId::new(stable(
                        "paragraph",
                        &format!("{part}#{}#{pi}", object.native_id),
                    ))
                    .expect("digest ID"),
                    runs: vec![],
                };
                for (ri, run) in runs.iter().enumerate() {
                    let id = RunId::new(stable(
                        "run",
                        &format!("{part}#{}#{pi}#{ri}", object.native_id),
                    ))
                    .expect("digest ID");
                    let kind = match run.kind {
                        SourceRunKind::Text => RetainedRunKind::Text,
                        SourceRunKind::Break => RetainedRunKind::Break,
                        SourceRunKind::Field => RetainedRunKind::Field,
                    };
                    let constraint =
                        if index.contains_signatures || !surface.text_edit_barriers.is_empty() {
                            Some(NativeEditConstraint::RetainedReferences)
                        } else {
                            run.edit_constraint
                                .map(|c| match c {
                                    SourceTextConstraint::CompatibilityBranch => {
                                        NativeEditConstraint::CompatibilityBranch
                                    }
                                    SourceTextConstraint::StructuredLeaf => {
                                        NativeEditConstraint::StructuredLeaf
                                    }
                                    SourceTextConstraint::DynamicField => {
                                        NativeEditConstraint::DynamicField
                                    }
                                    SourceTextConstraint::TimingReferences => {
                                        NativeEditConstraint::TimingReferences
                                    }
                                })
                                .or_else(|| {
                                    (!run.editable).then_some(NativeEditConstraint::StructuredLeaf)
                                })
                        };
                    native.runs.insert(
                        id.clone(),
                        NativeRunBinding {
                            paragraph: pi as u32,
                            run: ri as u32,
                            constraint,
                        },
                    );
                    paragraph.runs.push(RetainedTextRun {
                        id,
                        kind,
                        text: run.text.clone(),
                    });
                }
                paragraphs.push(paragraph);
            }
            bindings.objects.insert(id.clone(), native);
            let native_kind = match object.kind {
                SourceObjectKind::Shape => RetainedObjectKind::Shape,
                SourceObjectKind::Picture => RetainedObjectKind::Picture,
                SourceObjectKind::Group => RetainedObjectKind::Group,
                SourceObjectKind::Connector => RetainedObjectKind::Connector,
                SourceObjectKind::GraphicFrame => RetainedObjectKind::GraphicFrame,
            };
            d.objects.insert(
                id.clone(),
                Object {
                    id,
                    parent: object
                        .parent_group
                        .map(|id| ContainerId::Group(object_id(part, id)))
                        .unwrap_or_else(|| owner.clone()),
                    transform,
                    appearance: Default::default(),
                    accessibility: if matches!(
                        profile,
                        SourceBindingProfile::PresentationmlRetainedFieldsV4
                            | SourceBindingProfile::PresentationmlRetainedFieldsV5
                    ) {
                        object.accessibility.clone()
                    } else {
                        Default::default()
                    },
                    content: ObjectContent::RetainedSource {
                        native_kind,
                        children: children.remove(&object.native_id).unwrap_or_default(),
                        paragraphs,
                    },
                },
            );
        }
    }
    d.slide_order = index.slides.iter().map(|s| slide_id(&s.part)).collect();
    d.source_bindings = Some(bindings);
    Ok(d)
}
