//! Copy document-owned graphs in the kernel. The host supplies only the new
//! slide identity and destination order, never a partially rewritten graph.
use crate::EditError;
use mo_common::*;
use mo_presentation_model::*;
use mo_timeline::{Effect, StartCondition, TimeCondition, Timeline};
use std::collections::BTreeMap;

fn cancel(check: &dyn Fn() -> bool) -> Result<(), EditError> {
    if check() {
        Err(EditError::Cancelled)
    } else {
        Ok(())
    }
}

pub(crate) fn slide(
    document: &mut Document,
    source: &SlideId,
    destination: &SlideId,
    index: u32,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<(), EditError> {
    cancel(check)?;
    if document.slides.contains_key(destination) || index as usize > document.slide_order.len() {
        return Err(EditError::input(
            "duplicate slide destination exists or index is invalid",
        ));
    }
    if document.slides.len() >= limits.max_slides {
        return Err(EditError::input("duplicate slide exceeds slide budget"));
    }
    let mut slide = document
        .slides
        .get(source)
        .ok_or_else(|| EditError::input("duplicate slide source does not exist"))?
        .clone();
    let seed = digest(
        "musteroffice.duplicate-slide/1",
        &(&document.id, destination),
    )?;
    let identity = |kind: &str, old: &str| -> Result<String, EditError> {
        Ok(format!(
            "copy:{}",
            digest("musteroffice.duplicate-identity/1", &(&seed, kind, old))?
        ))
    };
    let mut pending = slide.objects.clone();
    let mut ids = BTreeMap::new();
    // Inspect the complete closure before copying its payload. Transaction
    // validation remains authoritative, including after preceding operations.
    while let Some(id) = pending.pop() {
        cancel(check)?;
        if ids.contains_key(&id) {
            return Err(EditError::input(
                "duplicate slide graph contains repeated ownership",
            ));
        }
        if document.objects.len().saturating_add(ids.len() + 1) > limits.max_objects {
            return Err(EditError::input("duplicate slide exceeds object budget"));
        }
        let object = document
            .objects
            .get(&id)
            .ok_or_else(|| EditError::input("duplicate slide object is missing"))?;
        match &object.content {
            ObjectContent::Group { children, .. } => pending.extend(children.iter().cloned()),
            ObjectContent::RetainedSource { .. } => {
                return Err(EditError::input(
                    "source-backed duplication requires native preservation",
                ));
            }
            _ => (),
        }
        let next =
            ObjectId::new(identity("object", id.as_str())?).expect("bounded derived identity");
        if document.objects.contains_key(&next) {
            return Err(EditError::input(
                "derived duplicate object identity already exists",
            ));
        }
        ids.insert(id, next);
    }
    let lookup = |id: &ObjectId| -> Result<ObjectId, EditError> {
        ids.get(id)
            .cloned()
            .ok_or_else(|| EditError::input("duplicate slide reference escapes owned graph"))
    };
    let mut objects = Vec::with_capacity(ids.len());
    for (old, next) in &ids {
        cancel(check)?;
        let mut object = document.objects[old].clone();
        object.id = next.clone();
        object.parent = match &object.parent {
            ContainerId::Slide(id) if id == source => ContainerId::Slide(destination.clone()),
            ContainerId::Group(id) => ContainerId::Group(lookup(id)?),
            _ => {
                return Err(EditError::input(
                    "duplicate slide object has a different owner",
                ));
            }
        };
        match &mut object.content {
            ObjectContent::Group { children, .. } => {
                for child in children {
                    *child = lookup(child)?;
                }
            }
            ObjectContent::Shape {
                text: Some(text), ..
            } => copy_text(text, &identity, check)?,
            ObjectContent::Table { table } => {
                // Row/column/cell identities are table-scoped. Covered cells keep
                // their origin links; document-scoped text gets new identities.
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        cancel(check)?;
                        if let Some(text) = &mut cell.text {
                            copy_text(text, &identity, check)?;
                        }
                    }
                }
            }
            ObjectContent::Connector { start, end } => {
                for endpoint in [start, end] {
                    if let ConnectorEndpoint::Attached { object, .. } = endpoint {
                        *object = lookup(object)?;
                    }
                }
            }
            ObjectContent::RetainedSource { .. } => unreachable!("closure rejects retained source"),
            ObjectContent::Shape { text: None, .. } | ObjectContent::Picture { .. } => (),
        }
        objects.push(object);
    }
    let timeline = document
        .timelines
        .get(source)
        .cloned()
        .map(|mut t| {
            copy_timeline(&mut t, &lookup, check)?;
            Ok::<_, EditError>(t)
        })
        .transpose()?;
    for id in &mut slide.objects {
        *id = lookup(id)?;
    }
    slide.id = destination.clone();
    cancel(check)?;
    document
        .slide_order
        .insert(index as usize, destination.clone());
    document.slides.insert(destination.clone(), slide);
    for object in objects {
        document.objects.insert(object.id.clone(), object);
    }
    if let Some(timeline) = timeline {
        document.timelines.insert(destination.clone(), timeline);
    }
    Ok(())
}

fn copy_text(
    body: &mut TextBody,
    identity: &dyn Fn(&str, &str) -> Result<String, EditError>,
    check: &dyn Fn() -> bool,
) -> Result<(), EditError> {
    for p in &mut body.paragraphs {
        cancel(check)?;
        p.id = ParagraphId::new(identity("paragraph", p.id.as_str())?)
            .expect("bounded derived identity");
        for run in &mut p.runs {
            cancel(check)?;
            run.id =
                RunId::new(identity("run", run.id.as_str())?).expect("bounded derived identity");
        }
    }
    Ok(())
}

fn copy_timeline(
    timeline: &mut Timeline,
    lookup: &dyn Fn(&ObjectId) -> Result<ObjectId, EditError>,
    check: &dyn Fn() -> bool,
) -> Result<(), EditError> {
    // Timing node identities are scoped to each slide's timeline; preserve all
    // dependency, sequence, preset and clock declarations verbatim.
    let condition = |c: &mut TimeCondition| -> Result<(), EditError> {
        cancel(check)?;
        if let TimeCondition::Click {
            target: Some(id), ..
        }
        | TimeCondition::Navigation {
            target: Some(id), ..
        } = c
        {
            *id = lookup(id)?;
        }
        Ok(())
    };
    let start = |s: &mut StartCondition| -> Result<(), EditError> {
        match s {
            StartCondition::Single(c) => condition(c),
            StartCondition::AnyOf { conditions } => conditions.iter_mut().try_for_each(&condition),
        }
    };
    for node in &mut timeline.nodes {
        cancel(check)?;
        let id = match &mut node.effect {
            Effect::Rotation { target, .. }
            | Effect::Scale { target, .. }
            | Effect::MotionLine { target, .. }
            | Effect::MotionPath { target, .. }
            | Effect::Fade { target, .. }
            | Effect::SetVisibility { target, .. } => target,
        };
        *id = lookup(id)?;
        start(&mut node.start)?;
        node.end_conditions.iter_mut().try_for_each(&condition)?;
    }
    if let Some(tree) = &mut timeline.tree {
        for c in &mut tree.containers {
            cancel(check)?;
            start(&mut c.start)?;
            c.end_conditions.iter_mut().try_for_each(&condition)?;
            if let Some(nav) = &mut c.navigation {
                nav.next_conditions
                    .iter_mut()
                    .chain(&mut nav.previous_conditions)
                    .try_for_each(&condition)?;
            }
        }
    }
    Ok(())
}
