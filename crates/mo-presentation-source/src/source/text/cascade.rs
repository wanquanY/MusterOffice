//! Source paragraph/run declaration inheritance. No host fonts, text layout,
//! field evaluation or drawing takes place here. See cascade::PROFILE.
mod chain;
mod properties;
mod table_text;
mod types;
use super::*;
use crate::{PptxError, cancelled, source::*, value};
pub use properties::{CharacterProperty, ParagraphProperty};
pub use table_text::{
    TableTextBinding, TableTextDeclaration, TableTextResolver, color_element, table_declaration,
};
pub use types::*;

/// Look up a selected declaration in the same inspected source. This intentionally
/// returns the complete native node, including any deferred/retained semantics.
pub fn declaration<'a>(
    index: &'a SourceIndex,
    reference: &TextStyleDeclaration,
) -> Result<&'a SourceTextNode, PptxError> {
    let (catalog, id) = declaration_catalog(index, reference)?;
    catalog
        .nodes
        .get(&id)
        .filter(|n| n.element == reference.element)
        .ok_or_else(conflict)
}

/// Access a direct selected declaration child without losing its source owner.
/// This never searches another template/catalog or accepts an unrelated ordinal.
pub fn declaration_child<'a>(
    index: &'a SourceIndex,
    reference: &TextStyleDeclaration,
    ordinal: u32,
) -> Result<(TextStyleDeclaration, &'a SourceTextNode), PptxError> {
    let parent = declaration(index, reference)?;
    if !parent.children.contains(&ordinal) {
        return Err(conflict());
    }
    let (catalog, id) = declaration_catalog(index, reference)?;
    let child = catalog
        .nodes
        .get(&ordinal)
        .filter(|n| n.parent == Some(id))
        .ok_or_else(conflict)?;
    Ok((
        TextStyleDeclaration {
            element: child.element,
            origin: reference.origin.at(ordinal),
        },
        child,
    ))
}
fn declaration_catalog<'a>(
    index: &'a SourceIndex,
    reference: &TextStyleDeclaration,
) -> Result<(&'a SourceTextCatalog, u32), PptxError> {
    let (catalog, id) = match &reference.origin {
        TextStyleOrigin::Object {
            object,
            source_ordinal,
        } => (
            &index.surfaces.get(&object.part).ok_or_else(conflict)?.text,
            *source_ordinal,
        ),
        TextStyleOrigin::Master {
            part,
            source_ordinal,
        } => (
            &index
                .surfaces
                .get(part)
                .filter(|s| s.kind == SurfaceKind::Master)
                .ok_or_else(conflict)?
                .text,
            *source_ordinal,
        ),
        TextStyleOrigin::Presentation {
            part,
            source_ordinal,
        } if part == &index.main_part => (&index.text, *source_ordinal),
        TextStyleOrigin::Theme {
            part,
            default_kind,
            source_ordinal,
        } => (
            &index
                .themes
                .get(part)
                .and_then(|t| t.text_defaults.as_ref())
                .and_then(|d| d.entries.get(default_kind))
                .ok_or_else(conflict)?
                .text,
            *source_ordinal,
        ),
        _ => return Err(conflict()),
    };
    Ok((catalog, id))
}

struct Budget<'a> {
    limits: TextCascadeLimits,
    check: &'a dyn Fn() -> bool,
    steps: usize,
    bytes: usize,
    runs: usize,
}
impl Budget<'_> {
    fn step(&mut self) -> Result<(), PptxError> {
        cancelled(self.check)?;
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(PptxError::Limit("text cascade steps"))?;
        if self.steps > self.limits.max_steps {
            return Err(PptxError::Limit("text cascade steps"));
        }
        Ok(())
    }
    fn bytes(&mut self, n: usize) -> Result<(), PptxError> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .ok_or(PptxError::Limit("text cascade bytes"))?;
        if self.bytes > self.limits.max_lexical_bytes {
            return Err(PptxError::Limit("text cascade bytes"));
        }
        Ok(())
    }
}
enum Failure {
    Unresolved(TextCascadeUnresolved),
    Abort(PptxError),
}
impl From<PptxError> for Failure {
    fn from(e: PptxError) -> Self {
        Self::Abort(e)
    }
}
impl From<TextCascadeUnresolved> for Failure {
    fn from(e: TextCascadeUnresolved) -> Self {
        Self::Unresolved(e)
    }
}
fn conflict() -> PptxError {
    PptxError::SourceConflict("text cascade differs from inspected source".into())
}

#[derive(Clone)]
struct Location<'a> {
    catalog: &'a SourceTextCatalog,
    id: u32,
    origin: TextStyleOrigin,
}
impl<'a> Location<'a> {
    fn node(&self) -> Result<&'a SourceTextNode, PptxError> {
        self.catalog.nodes.get(&self.id).ok_or_else(conflict)
    }
    fn checked(&self, budget: &mut Budget<'_>) -> Result<&'a SourceTextNode, Failure> {
        budget.step()?;
        let n = self.node()?;
        if let Some(at) = n.retained_ordinals.first() {
            return Err(TextCascadeUnresolved::RetainedContent {
                origin: self.origin.at(*at),
            }
            .into());
        }
        Ok(n)
    }
    fn children(&self, budget: &mut Budget<'_>) -> Result<Vec<Self>, PptxError> {
        let mut result = vec![];
        let mut seen = std::collections::BTreeSet::new();
        for id in &self.node()?.children {
            budget.step()?;
            let n = self.catalog.nodes.get(id).ok_or_else(conflict)?;
            if n.parent != Some(self.id) || !seen.insert(*id) {
                return Err(conflict());
            }
            budget.bytes(self.origin.bytes() + 32)?;
            result.push(Self {
                catalog: self.catalog,
                id: *id,
                origin: self.origin.at(*id),
            });
        }
        Ok(result)
    }
    fn child(
        &self,
        element: NativeTextElement,
        budget: &mut Budget<'_>,
    ) -> Result<Option<Self>, PptxError> {
        let mut result = None;
        for c in self.children(budget)? {
            if c.node()?.element == element && result.replace(c).is_some() {
                return Err(conflict());
            }
        }
        Ok(result)
    }
}

fn declarations<S: Ord>(
    location: &Location<'_>,
    slots: &mut BTreeMap<S, TextStyleDeclaration>,
    classify: fn(NativeTextElement) -> Option<S>,
    budget: &mut Budget<'_>,
) -> Result<(), Failure> {
    for c in location.children(budget)? {
        let element = c.node()?.element;
        if let Some(slot) = classify(element)
            && let std::collections::btree_map::Entry::Vacant(e) = slots.entry(slot)
        {
            // Descendant paint/font/relationship semantics remain explicitly
            // deferred. Never call a selected source reference rendered.
            c.checked(budget)?;
            budget.bytes(c.origin.bytes() + 32)?;
            e.insert(TextStyleDeclaration {
                element,
                origin: c.origin,
            });
        }
    }
    Ok(())
}
#[derive(Clone)]
enum Layer<'a> {
    Native(Location<'a>),
    Table(table_text::CharacterLayer<'a>),
}
fn character(
    locations: &[Layer<'_>],
    budget: &mut Budget<'_>,
) -> Result<CascadedCharacterStyle, Failure> {
    let mut style = CascadedCharacterStyle::default();
    for layer in locations {
        let Layer::Native(at) = layer else {
            if let Layer::Table(table) = layer {
                table.apply(&mut style, budget)?;
            }
            continue;
        };
        let n = at.checked(budget)?;
        let SourceTextValue::Character { attributes } = &n.value else {
            return Err(conflict().into());
        };
        properties::character_bytes(attributes, budget)?;
        properties::character(
            &mut style.attributes,
            attributes,
            &mut style.origins,
            &at.origin,
            budget,
        )?;
        declarations(
            at,
            &mut style.declarations,
            properties::character_slot,
            budget,
        )?;
    }
    properties::character(
        &mut style.attributes,
        &properties::character_defaults(),
        &mut style.origins,
        &TextStyleOrigin::ProfileDefault {},
        budget,
    )?;
    Ok(style)
}
struct Scope<'a> {
    cell: Option<table::SourceCellAddress>,
    table: Option<table_text::CharacterLayer<'a>>,
}
fn paragraph<'a>(
    location: &Location<'a>,
    object: &SourceObject,
    paragraph_index: usize,
    context: &chain::Context<'a>,
    scope: &Scope<'a>,
    levels: &mut BTreeMap<i32, Vec<Layer<'a>>>,
    budget: &mut Budget<'_>,
) -> Result<CascadedParagraph, Failure> {
    location.checked(budget)?;
    let local = location.child(NativeTextElement::PPr, budget)?;
    let level = if let Some(p) = &local {
        let SourceTextValue::Paragraph { attributes } = &p.checked(budget)?.value else {
            return Err(conflict().into());
        };
        attributes.level.unwrap_or(0)
    } else {
        0
    };
    if !(0..=8).contains(&level) {
        return Err(conflict().into());
    }
    let mut parents = vec![];
    if let Some(p) = local {
        parents.push(Layer::Native(p));
    }
    if let std::collections::btree_map::Entry::Vacant(e) = levels.entry(level) {
        e.insert(context.level(level, scope.cell, scope.table.as_ref(), budget)?);
    }
    for at in &levels[&level] {
        budget.bytes(match at {
            Layer::Native(at) => at.origin.bytes() + 32,
            Layer::Table(t) => t.source.lexical_bytes() + 128,
        })?;
        parents.push(at.clone());
    }
    let mut attributes = SourceTextParagraphAttributes::default();
    let mut origins = BTreeMap::new();
    let mut selected = BTreeMap::new();
    let mut characters = vec![];
    for layer in &parents {
        let Layer::Native(at) = layer else {
            characters.push(layer.clone());
            continue;
        };
        let n = at.checked(budget)?;
        let SourceTextValue::Paragraph { attributes: from } = &n.value else {
            return Err(conflict().into());
        };
        if let Some(c) = &from.default_tab_size {
            budget.bytes(c.lexical().len())?;
        }
        // lvl selects a local paragraph's style; lvl on lvlNpPr is ignored.
        let mut from = *from.clone();
        if !std::ptr::eq(at.catalog, location.catalog) || n.parent != Some(location.id) {
            from.level = None;
        }
        properties::paragraph(&mut attributes, &from, &mut origins, &at.origin, budget)?;
        declarations(at, &mut selected, properties::paragraph_slot, budget)?;
        if let Some(c) = at.child(NativeTextElement::DefRPr, budget)? {
            characters.push(Layer::Native(c));
        }
    }
    properties::paragraph(
        &mut attributes,
        &properties::paragraph_defaults(),
        &mut origins,
        &TextStyleOrigin::ProfileDefault {},
        budget,
    )?;
    let mut runs = vec![];
    for c in location.children(budget)? {
        let n = c.node()?;
        let kind = match n.element {
            NativeTextElement::R => SourceRunKind::Text,
            NativeTextElement::Br => SourceRunKind::Break,
            NativeTextElement::Fld => SourceRunKind::Field,
            NativeTextElement::PPr | NativeTextElement::EndParaRPr => continue,
            _ => return Err(conflict().into()),
        };
        c.checked(budget)?;
        budget.runs = budget
            .runs
            .checked_add(1)
            .ok_or(PptxError::Limit("text cascade runs"))?;
        if budget.runs > budget.limits.max_runs {
            return Err(PptxError::Limit("text cascade runs").into());
        }
        let run = object
            .paragraphs
            .get(paragraph_index)
            .and_then(|p| p.get(runs.len()))
            .ok_or_else(conflict)?;
        if run.kind != kind {
            return Err(conflict().into());
        }
        if let Some(at) = c.child(NativeTextElement::PPr, budget)? {
            return Err(TextCascadeUnresolved::FieldParagraph { origin: at.origin }.into());
        }
        // Check unresolved text-leaf attributes before advertising a usable
        // source binding. Explicit xml:space preservation retains every scalar.
        if let Some(t) = c.child(NativeTextElement::T, budget)? {
            t.checked(budget)?;
        }
        let local = c.child(NativeTextElement::RPr, budget)?;
        let mut chain = local.into_iter().map(Layer::Native).collect::<Vec<_>>();
        chain.extend(characters.iter().cloned());
        let style = character(&chain, budget)?;
        runs.push(CascadedTextRun {
            source_ordinal: c.id,
            run: runs.len() as u32,
            kind,
            style,
        });
    }
    if object.paragraphs[paragraph_index].len() != runs.len() {
        return Err(conflict().into());
    }
    let mut end = location
        .child(NativeTextElement::EndParaRPr, budget)?
        .into_iter()
        .map(Layer::Native)
        .collect::<Vec<_>>();
    end.extend(characters);
    let end_style = character(&end, budget)?;
    Ok(CascadedParagraph {
        source_ordinal: location.id,
        attributes,
        origins,
        declarations: selected,
        runs,
        end_style,
    })
}

/// Library entry point for the forthcoming source text compiler. Bind the index
/// to the caller's expected source revision; this does not authenticate an index
/// supplied by an untrusted party. Hosts should inspect the actual package.
pub fn resolve(
    index: &SourceIndex,
    expected_source_sha256: &mo_common::Digest,
    object: &SourceObjectRef,
    limits: TextCascadeLimits,
    check: &dyn Fn() -> bool,
) -> Result<TextCascadeOutcome, PptxError> {
    cancelled(check)?;
    if &index.source_sha256 != expected_source_sha256 {
        return Err(PptxError::SourceConflict(
            "text cascade source digest differs".into(),
        ));
    }
    let mut budget = Budget {
        limits,
        check,
        steps: 0,
        bytes: 0,
        runs: 0,
    };
    let result = (|| -> Result<CascadedText, Failure> {
        let context = chain::Context::new(index, object, &mut budget)?;
        let root = context
            .target
            .body(None)?
            .ok_or(TextCascadeUnresolved::NoTextBody {})?;
        root.checked(&mut budget)?;
        let font_reference = context.font_reference(&mut budget)?;
        let mut paragraphs = vec![];
        let mut levels = BTreeMap::new();
        for p in root.children(&mut budget)? {
            if p.node()?.element != NativeTextElement::P {
                continue;
            }
            if paragraphs.len() >= limits.max_paragraphs {
                return Err(PptxError::Limit("text cascade paragraphs").into());
            }
            if paragraphs.len() >= context.target.object.paragraphs.len() {
                return Err(conflict().into());
            }
            paragraphs.push(paragraph(
                &p,
                context.target.object,
                paragraphs.len(),
                &context,
                &Scope {
                    cell: None,
                    table: None,
                },
                &mut levels,
                &mut budget,
            )?);
        }
        if paragraphs.len() != context.target.object.paragraphs.len() || paragraphs.is_empty() {
            return Err(conflict().into());
        }
        budget.bytes(object.part.len() + 256)?;
        Ok(CascadedText {
            profile: PROFILE.into(),
            source_sha256: index.source_sha256.clone(),
            object: object.clone(),
            cell: None,
            paragraph_start: 0,
            font_reference,
            paragraphs,
        })
    })();
    match result {
        Ok(text) => Ok(TextCascadeOutcome::Cascaded {
            text: Box::new(text),
        }),
        Err(Failure::Unresolved(reason)) => Ok(TextCascadeOutcome::Unresolved { reason }),
        Err(Failure::Abort(error)) => Err(error),
    }
}
