//! Page-level semantic dependencies shared by editing and compilation. Metadata
//! and the timeline do not affect the static editor layout. Declarations remain
//! inherited; this index selects dependencies without flattening their values.
use crate::*;
use mo_common::{CanonicalError, Digest, FontId, ObjectId, ResourceId, SlideId, digest};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq, Serialize)]
struct VisualObject<'a> {
    binding: Option<&'a NativeObjectBinding>,
    parent: &'a ContainerId,
    transform: &'a Option<Transform>,
    appearance: &'a Appearance,
    content: &'a ObjectContent,
}

/// Construct from an admitted document. Missing references return None; invalid
/// graphs are never repaired here or allowed to loop. Callers must still run
/// model validation at the admission boundary, including before cache hits.
#[derive(PartialEq, Eq, Serialize)]
pub struct PageDependencies<'a> {
    source_binding: Option<PageSourceBinding<'a>>,
    page_size: Size,
    slide: &'a SlideId,
    hidden: bool,
    background: &'a Inherited<Fill>,
    roots: &'a [ObjectId],
    layout: Option<SurfaceDependencies<'a, mo_common::LayoutId>>,
    master: Option<SurfaceDependencies<'a, mo_common::MasterId>>,
    theme: Option<ThemeDependencies<'a>>,
    objects: BTreeMap<&'a ObjectId, VisualObject<'a>>,
    fonts: BTreeMap<&'a FontId, &'a FontFace>,
    resources: BTreeMap<&'a ResourceId, &'a Resource>,
}

impl<'a> PageDependencies<'a> {
    pub fn new(document: &'a Document, slide: &'a SlideId) -> Option<Self> {
        let s = document.slides.get(slide)?;
        let layout = match &s.layout {
            Some(id) => Some(document.layouts.get(id)?),
            None => None,
        };
        let master = match layout {
            Some(l) => Some(document.masters.get(&l.master)?),
            None => None,
        };
        let theme = match master {
            Some(m) => Some(document.themes.get(&m.theme)?),
            None => None,
        };
        let mut dependencies = Self {
            source_binding: document
                .source_bindings
                .as_ref()
                .map(|b| PageSourceBinding {
                    profile: &b.profile,
                    resource: &b.resource,
                    slide: b.slides.get(slide),
                    layout: layout.and_then(|l| b.layouts.get(&l.id)),
                    master: master.and_then(|m| b.masters.get(&m.id)),
                    theme: theme.and_then(|t| b.themes.get(&t.id)),
                }),
            page_size: document.page_size,
            slide,
            hidden: s.hidden,
            background: &s.background,
            roots: &s.objects,
            layout: layout.map(|l| SurfaceDependencies {
                id: &l.id,
                roots: &l.objects,
                background: &l.background,
                text: &l.default_text,
            }),
            master: master.map(|m| SurfaceDependencies {
                id: &m.id,
                roots: &m.objects,
                background: &m.background,
                text: &m.default_text,
            }),
            theme: theme.map(|t| ThemeDependencies {
                id: &t.id,
                colors: &t.colors,
                text: &t.default_text,
            }),
            objects: BTreeMap::new(),
            fonts: BTreeMap::new(),
            resources: BTreeMap::new(),
        };
        let mut fonts = BTreeSet::new();
        for style in [
            layout.map(|l| &l.default_text),
            master.map(|m| &m.default_text),
            theme.map(|t| &t.default_text),
        ]
        .into_iter()
        .flatten()
        {
            font(style, &mut fonts);
        }
        let mut pending: Vec<_> = s
            .objects
            .iter()
            .chain(layout.into_iter().flat_map(|l| &l.objects))
            .chain(master.into_iter().flat_map(|m| &m.objects))
            .collect();
        while let Some(id) = pending.pop() {
            if dependencies.objects.contains_key(id) {
                continue;
            }
            let object = document.objects.get(id)?;
            dependencies.objects.insert(
                id,
                VisualObject {
                    binding: document
                        .source_bindings
                        .as_ref()
                        .and_then(|b| b.objects.get(id)),
                    parent: &object.parent,
                    transform: &object.transform,
                    appearance: &object.appearance,
                    content: &object.content,
                },
            );
            match &object.content {
                ObjectContent::Group { children, .. }
                | ObjectContent::RetainedSource { children, .. } => pending.extend(children),
                ObjectContent::Picture { resource, .. } => {
                    dependencies
                        .resources
                        .insert(resource, document.resources.get(resource)?);
                }
                ObjectContent::Shape {
                    text: Some(text), ..
                } => {
                    font(&text.style, &mut fonts);
                    for paragraph in &text.paragraphs {
                        font(&paragraph.default_run_style, &mut fonts);
                        for run in &paragraph.runs {
                            font(&run.style, &mut fonts);
                        }
                    }
                }
                ObjectContent::Connector { start, end } => {
                    for endpoint in [start, end] {
                        if let ConnectorEndpoint::Attached { object, .. } = endpoint {
                            pending.push(object);
                        }
                    }
                }
                ObjectContent::Shape { text: None, .. } => {}
            }
        }
        for id in fonts {
            let face = document.fonts.get(id)?;
            dependencies.fonts.insert(id, face);
            dependencies
                .resources
                .insert(&face.resource, document.resources.get(&face.resource)?);
        }
        // Retained source may contain unprojected page dependencies. Its identity
        // is part of every key until import provides a narrower native closure.
        dependencies.resources.extend(
            document
                .resources
                .iter()
                .filter(|(_, r)| r.kind == ResourceKind::SourcePackage),
        );
        Some(dependencies)
    }

    pub fn digest(&self) -> Result<Digest, CanonicalError> {
        digest("musteroffice.presentation.page-dependencies/1", self)
    }
}

fn font<'a>(style: &'a CharacterStyle, fonts: &mut BTreeSet<&'a FontId>) {
    if let Inherited::Value(id) = &style.font {
        fonts.insert(id);
    }
}

#[derive(PartialEq, Eq, Serialize)]
struct SurfaceDependencies<'a, I> {
    id: &'a I,
    roots: &'a [ObjectId],
    background: &'a Inherited<Fill>,
    text: &'a CharacterStyle,
}
#[derive(PartialEq, Eq, Serialize)]
struct ThemeDependencies<'a> {
    id: &'a mo_common::ThemeId,
    colors: &'a BTreeMap<ThemeColor, Rgba>,
    text: &'a CharacterStyle,
}

#[derive(PartialEq, Eq, Serialize)]
struct PageSourceBinding<'a> {
    profile: &'a SourceBindingProfile,
    resource: &'a ResourceId,
    slide: Option<&'a String>,
    layout: Option<&'a String>,
    master: Option<&'a String>,
    theme: Option<&'a String>,
}
