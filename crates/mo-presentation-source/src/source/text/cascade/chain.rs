use super::*;
use crate::source::theme::SourceThemeDefaultKind;
use NativeTextElement as N;

pub(super) struct Object<'a> {
    part: &'a str,
    surface: &'a SourceSurface,
    pub object: &'a SourceObject,
}
impl<'a> Object<'a> {
    fn font_reference(&self, budget: &mut Budget<'_>) -> Result<Option<Location<'a>>, PptxError> {
        let mut found = None;
        for root in &self.surface.text.roots {
            budget.step()?;
            if root.owner != Some(self.object.native_id) {
                continue;
            }
            let node = self
                .surface
                .text
                .nodes
                .get(&root.source_ordinal)
                .ok_or_else(conflict)?;
            if node.element == N::FontRef {
                if node.parent.is_some() || found.is_some() {
                    return Err(conflict());
                }
                found = Some(Location {
                    catalog: &self.surface.text,
                    id: root.source_ordinal,
                    origin: TextStyleOrigin::Object {
                        object: self.reference(),
                        source_ordinal: root.source_ordinal,
                    },
                });
            }
        }
        Ok(found)
    }
    pub fn reference(&self) -> SourceObjectRef {
        SourceObjectRef {
            part: self.part.into(),
            native_id: self.object.native_id,
        }
    }
    pub fn body(
        &self,
        cell: Option<table::SourceCellAddress>,
    ) -> Result<Option<Location<'a>>, PptxError> {
        let Some(body) = bind_body(self.object, &self.surface.text, cell)? else {
            return Ok(None);
        };
        let id = body.root.source_ordinal;
        Ok(Some(Location {
            catalog: &self.surface.text,
            id,
            origin: TextStyleOrigin::Object {
                object: self.reference(),
                source_ordinal: id,
            },
        }))
    }
    fn styles(
        &self,
        level: i32,
        template: bool,
        cell: Option<table::SourceCellAddress>,
        budget: &mut Budget<'_>,
    ) -> Result<Vec<Location<'a>>, Failure> {
        let Some(body) = self.body(cell)? else {
            return Ok(vec![]);
        };
        body.checked(budget)?;
        let mut result = vec![];
        if template {
            let mut matched = false;
            for p in body.children(budget)? {
                if p.node()?.element != N::P {
                    continue;
                }
                let at = p.child(N::PPr, budget)?;
                let actual_level = if let Some(at) = &at {
                    let SourceTextValue::Paragraph { attributes } = &at.node()?.value else {
                        return Err(conflict().into());
                    };
                    attributes.level.unwrap_or(0)
                } else {
                    0
                };
                if actual_level == level {
                    if matched {
                        return Err(TextCascadeUnresolved::AmbiguousTemplateParagraph {
                            object: self.reference(),
                            level,
                        }
                        .into());
                    }
                    matched = true;
                    p.checked(budget)?;
                    result.extend(at);
                }
            }
        }
        if let Some(list) = body.child(N::LstStyle, budget)? {
            result.extend(list_level(&list, level, budget)?);
        }
        Ok(result)
    }
}
fn object<'a>(
    index: &'a SourceIndex,
    reference: &SourceObjectRef,
    budget: &mut Budget<'_>,
) -> Result<Object<'a>, PptxError> {
    let (part, surface) = index
        .surfaces
        .get_key_value(&reference.part)
        .ok_or_else(|| value("textCascade.object", "surface not in source"))?;
    let mut found = None;
    for object in &surface.objects {
        budget.step()?;
        if object.native_id == reference.native_id && found.replace(object).is_some() {
            return Err(conflict());
        }
    }
    Ok(Object {
        part,
        surface,
        object: found.ok_or_else(|| value("textCascade.object", "object not in surface"))?,
    })
}
fn surface<'a>(
    index: &'a SourceIndex,
    part: &str,
    kind: SurfaceKind,
) -> Result<(&'a str, &'a SourceSurface), PptxError> {
    let (part, surface) = index.surfaces.get_key_value(part).ok_or_else(conflict)?;
    if surface.kind != kind {
        return Err(conflict());
    }
    Ok((part, surface))
}
fn parent<'a>(
    index: &'a SourceIndex,
    object: &Object<'_>,
    budget: &mut Budget<'_>,
) -> Result<Option<Object<'a>>, Failure> {
    match &object.object.resolution.placeholder_match {
        SourcePlaceholderMatch::Matched { target, .. } => {
            let (link, kind) = match object.surface.kind {
                SurfaceKind::Slide => (&object.surface.links.layout, SurfaceKind::Layout),
                SurfaceKind::Layout => (&object.surface.links.master, SurfaceKind::Master),
                SurfaceKind::Master => return Err(conflict().into()),
            };
            if link.as_deref() != Some(&target.part) {
                return Err(conflict().into());
            }
            surface(index, &target.part, kind)?;
            Ok(Some(self::object(index, target, budget)?))
        }
        SourcePlaceholderMatch::NotPlaceholder
        | SourcePlaceholderMatch::Detached
        | SourcePlaceholderMatch::Master => Ok(None),
        matching => Err(TextCascadeUnresolved::Placeholder {
            object: object.reference(),
            matching: matching.clone(),
        }
        .into()),
    }
}
fn root<'a>(
    catalog: &'a SourceTextCatalog,
    element: N,
    origin: TextStyleOrigin,
    budget: &mut Budget<'_>,
) -> Result<Option<Location<'a>>, PptxError> {
    let mut result = None;
    for r in &catalog.roots {
        budget.step()?;
        let n = catalog.nodes.get(&r.source_ordinal).ok_or_else(conflict)?;
        if n.element == element {
            if r.owner.is_some() || n.parent.is_some() || result.is_some() {
                return Err(conflict());
            }
            result = Some(Location {
                catalog,
                id: r.source_ordinal,
                origin: origin.at(r.source_ordinal),
            });
        }
    }
    Ok(result)
}
fn list_level<'a>(
    list: &Location<'a>,
    level: i32,
    budget: &mut Budget<'_>,
) -> Result<Option<Location<'a>>, Failure> {
    list.checked(budget)?;
    let element = [
        N::Lvl1pPr,
        N::Lvl2pPr,
        N::Lvl3pPr,
        N::Lvl4pPr,
        N::Lvl5pPr,
        N::Lvl6pPr,
        N::Lvl7pPr,
        N::Lvl8pPr,
        N::Lvl9pPr,
    ]
    .get(level as usize)
    .copied()
    .ok_or_else(conflict)?;
    // Office ignores defPPr. Other levels and their retained content do not
    // affect this paragraph; no fallback to level 1 is invented here.
    Ok(list.child(element, budget)?)
}
pub(super) struct Context<'a> {
    index: &'a SourceIndex,
    pub target: Object<'a>,
    layout: Option<Object<'a>>,
    master_object: Option<Object<'a>>,
    master: Option<(&'a str, &'a SourceSurface)>,
}
impl<'a> Context<'a> {
    pub fn font_reference(
        &self,
        budget: &mut Budget<'_>,
    ) -> Result<Option<TextStyleDeclaration>, Failure> {
        let finish = |at: Location<'_>, budget: &mut Budget<'_>| -> Result<_, Failure> {
            at.checked(budget)?;
            budget.bytes(at.origin.bytes() + 32)?;
            Ok(Some(TextStyleDeclaration {
                element: N::FontRef,
                origin: at.origin,
            }))
        };
        for object in std::iter::once(&self.target).chain(self.layout.iter()) {
            if let Some(at) = object.font_reference(budget)? {
                return finish(at, budget);
            }
        }
        if let Some(base) = &self.target.surface.effective_theme.base {
            let theme = self.index.themes.get(&base.part).ok_or_else(conflict)?;
            if let Some(defaults) = &theme.text_defaults {
                for kind in [
                    SourceThemeDefaultKind::Text,
                    SourceThemeDefaultKind::Line,
                    SourceThemeDefaultKind::Shape,
                ] {
                    if let Some(entry) = defaults.entries.get(&kind) {
                        let origin = TextStyleOrigin::Theme {
                            part: base.part.clone(),
                            default_kind: kind,
                            source_ordinal: entry.source_ordinal,
                        };
                        if let Some(at) = root(&entry.text, N::FontRef, origin, budget)? {
                            return finish(at, budget);
                        }
                    }
                }
            }
        }
        if let Some(master) = &self.master_object
            && let Some(at) = master.font_reference(budget)?
        {
            return finish(at, budget);
        }
        Ok(None)
    }
    pub fn new(
        index: &'a SourceIndex,
        reference: &SourceObjectRef,
        budget: &mut Budget<'_>,
    ) -> Result<Self, Failure> {
        let target = object(index, reference, budget)?;
        Self::from_target(index, target, budget)
    }
    pub fn prepared_table(
        table: &crate::source::prepared::PreparedSourceTable<'a>,
        budget: &mut Budget<'_>,
    ) -> Result<Self, Failure> {
        budget.step()?;
        Self::from_target(
            table.index(),
            Object {
                part: table.part(),
                surface: table.surface(),
                object: table.object(),
            },
            budget,
        )
    }
    fn from_target(
        index: &'a SourceIndex,
        target: Object<'a>,
        budget: &mut Budget<'_>,
    ) -> Result<Self, Failure> {
        let inherited = if target.object.table.is_some() {
            None
        } else {
            parent(index, &target, budget)?
        };
        let (layout, master_object) = if target.surface.kind == SurfaceKind::Slide {
            let master = inherited
                .as_ref()
                .map(|l| parent(index, l, budget))
                .transpose()?
                .flatten();
            (inherited, master)
        } else {
            (None, inherited)
        };
        // A non-placeholder still gets the linked master's otherStyle.
        let master = match target.surface.kind {
            SurfaceKind::Master => Some((target.part, target.surface)),
            SurfaceKind::Layout => target
                .surface
                .links
                .master
                .as_deref()
                .map(|p| surface(index, p, SurfaceKind::Master))
                .transpose()?,
            SurfaceKind::Slide => {
                let layout = target
                    .surface
                    .links
                    .layout
                    .as_deref()
                    .map(|p| surface(index, p, SurfaceKind::Layout))
                    .transpose()?;
                layout
                    .and_then(|(_, s)| s.links.master.as_deref())
                    .map(|p| surface(index, p, SurfaceKind::Master))
                    .transpose()?
            }
        };
        Ok(Self {
            index,
            target,
            layout,
            master_object,
            master,
        })
    }
    pub fn level(
        &self,
        level: i32,
        cell: Option<table::SourceCellAddress>,
        table: Option<&table_text::CharacterLayer<'a>>,
        budget: &mut Budget<'_>,
    ) -> Result<Vec<Layer<'a>>, Failure> {
        let mut result: Vec<Layer<'a>> = self
            .target
            .styles(level, false, cell, budget)?
            .into_iter()
            .map(Layer::Native)
            .collect();
        if let Some(table) = table {
            budget.bytes(table.source.lexical_bytes() + 128)?;
            result.push(Layer::Table(table.clone()));
        }
        let mut inherited = vec![];
        if let Some(layout) = &self.layout {
            inherited.extend(layout.styles(level, true, None, budget)?);
        }
        if let Some(base) = &self.target.surface.effective_theme.base {
            let theme = self.index.themes.get(&base.part).ok_or_else(conflict)?;
            if let Some(defaults) = &theme.text_defaults {
                for kind in [
                    SourceThemeDefaultKind::Text,
                    SourceThemeDefaultKind::Line,
                    SourceThemeDefaultKind::Shape,
                ] {
                    budget.step()?;
                    let origin = TextStyleOrigin::Theme {
                        part: base.part.clone(),
                        default_kind: kind,
                        source_ordinal: defaults.source_ordinal,
                    };
                    if let Some(at) = defaults.retained_ordinals.first() {
                        return Err(TextCascadeUnresolved::RetainedContent {
                            origin: origin.at(*at),
                        }
                        .into());
                    }
                    if let Some(entry) = defaults.entries.get(&kind) {
                        if let Some(at) = entry.retained_ordinals.first() {
                            return Err(TextCascadeUnresolved::RetainedContent {
                                origin: origin.at(*at),
                            }
                            .into());
                        }
                        if let Some(list) = root(&entry.text, N::LstStyle, origin, budget)? {
                            inherited.extend(list_level(&list, level, budget)?);
                        }
                    }
                }
            }
        }
        if let Some(master) = &self.master_object {
            inherited.extend(master.styles(level, true, None, budget)?);
        }
        if let Some((part, master)) = self.master
            && let Some(styles) = root(
                &master.text,
                N::TxStyles,
                TextStyleOrigin::Master {
                    part: part.into(),
                    source_ordinal: 0,
                },
                budget,
            )?
        {
            styles.checked(budget)?;
            let kind = if cell.is_some() {
                // A cell never inherits a shape placeholder's template body.
                N::OtherStyle
            } else {
                match self
                    .target
                    .object
                    .placeholder
                    .as_ref()
                    .map(SourcePlaceholder::effective_kind)
                    .and_then(crate::source::inheritance::master_kind)
                {
                    Some(PlaceholderKind::Title) => N::TitleStyle,
                    Some(PlaceholderKind::Body) => N::BodyStyle,
                    _ => N::OtherStyle,
                }
            };
            if let Some(list) = styles.child(kind, budget)? {
                inherited.extend(list_level(&list, level, budget)?);
            }
        }
        if let Some(list) = root(
            &self.index.text,
            N::DefaultTextStyle,
            TextStyleOrigin::Presentation {
                part: self.index.main_part.clone(),
                source_ordinal: 0,
            },
            budget,
        )? {
            inherited.extend(list_level(&list, level, budget)?);
        }
        result.extend(inherited.into_iter().map(Layer::Native));
        Ok(result)
    }
}
