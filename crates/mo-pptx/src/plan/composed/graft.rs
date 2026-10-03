//! Graft only the authored slide dependency closure. Source bytes and unknown
//! relationships are preserved; names and relationship IDs cannot collide.
use crate::*;
use mo_common::Digest;
use mo_opc::{
    GraphPart, Package, PackageLimits, PartName, ReaderAt, Relationship, RelationshipSource,
    RelationshipTarget, RewritePlan, VerifiedPackage,
};
use mo_presentation_source::{author::NativeBindings, source::SourceIndex};
use mo_xml::{ExpandedName, XmlEvent};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn prefix(hash: &Digest) -> String {
    format!("/ppt/musterOffice/{hash}")
}

pub(super) fn append<R: ReaderAt, A: ReaderAt>(
    source: &Package<R>,
    author: &Package<A>,
    bindings: &NativeBindings,
    original: &SourceIndex,
    limits: PackageLimits,
    check: &dyn Fn() -> bool,
) -> Result<VerifiedPackage<Vec<u8>>, PptxError> {
    let prefix = prefix(author.sha256());
    let relocated = |part: &PartName| PartName::new(format!("{prefix}{part}"));
    let mut closure = BTreeSet::new();
    let mut pending: Vec<_> = bindings
        .slides
        .iter()
        .map(|s| s.part.clone())
        .chain(bindings.masters.iter().map(|m| m.part.clone()))
        .collect();
    while let Some(part) = pending.pop() {
        cancelled(check)?;
        if !closure.insert(part.clone()) {
            continue;
        }
        if closure.len() > limits.max_parts {
            return Err(PptxError::Limit("appended graph parts"));
        }
        for relation in author
            .relationships()
            .get(&RelationshipSource::Part(part))
            .into_iter()
            .flatten()
        {
            if let RelationshipTarget::Internal { part, .. } = &relation.resolved {
                pending.push(part.clone());
            }
        }
    }
    let mut additions = Vec::new();
    let mut relationships = BTreeMap::new();
    let mut total = 0u64;
    for part in &closure {
        cancelled(check)?;
        let info = author
            .parts()
            .get(part)
            .ok_or_else(|| conflict("authored closure part is absent"))?;
        total = total
            .checked_add(info.byte_length)
            .ok_or(PptxError::Limit("appended graph bytes"))?;
        if total > limits.max_inflated_bytes {
            return Err(PptxError::Limit("appended graph bytes"));
        }
        let name = relocated(part)?;
        additions.push(GraphPart {
            name: name.clone(),
            content_type: info.content_type.clone(),
            bytes: author.read_part(part, limits.max_part_bytes, check)?,
        });
        let owner = RelationshipSource::Part(name);
        let mut moved = Vec::new();
        for r in author
            .relationships()
            .get(&RelationshipSource::Part(part.clone()))
            .into_iter()
            .flatten()
        {
            let target = match &r.resolved {
                RelationshipTarget::Internal { part, fragment } => {
                    if !closure.contains(part) {
                        return Err(conflict("incomplete authored closure"));
                    }
                    format!(
                        "{}{}",
                        relocated(part)?,
                        fragment
                            .as_ref()
                            .map(|s| format!("#{s}"))
                            .unwrap_or_default()
                    )
                }
                RelationshipTarget::External => r.target.clone(),
            };
            moved.push(Relationship::new(
                &owner,
                r.id.clone(),
                r.relationship_type.clone(),
                target,
                r.resolved == RelationshipTarget::External,
            )?);
        }
        relationships.insert(owner, moved);
    }
    let main = PartName::new(original.main_part.clone())?;
    let owner = RelationshipSource::Part(main.clone());
    let mut relation_ids: BTreeSet<_> = source
        .relationships()
        .get(&owner)
        .into_iter()
        .flatten()
        .map(|r| r.id.clone())
        .collect();
    let mut xml = source.read_part(&main, limits.xml.max_bytes as u64, check)?;
    let mut masters = BTreeSet::new();
    let mut slides = BTreeSet::new();
    mo_xml::scan_with_control(&xml, limits.xml, check, |event| {
        if let XmlEvent::Start { element, .. } = event {
            let target = if element.name.is(P, "sldMasterId") {
                Some(&mut masters)
            } else if element.name.is(P, "sldId") {
                Some(&mut slides)
            } else {
                None
            };
            if let Some(ids) = target {
                let id = element
                    .attribute("id")
                    .and_then(|s| s.parse::<u32>().ok())
                    .ok_or_else(|| {
                        mo_xml::XmlError::Malformed("source page or master ID".into())
                    })?;
                ids.insert(id);
            }
        }
        Ok(())
    })?;
    let mut main_relations = Vec::new();
    let mut relation_index = 0usize;
    for (kind, part) in bindings
        .masters
        .iter()
        .map(|m| ("sldMasterId", &m.part))
        .chain(bindings.slides.iter().map(|s| ("sldId", &s.part)))
    {
        cancelled(check)?;
        let relation_id = loop {
            relation_index = relation_index
                .checked_add(1)
                .ok_or(PptxError::Limit("appended relation IDs"))?;
            let id = format!("moAppend{relation_index}");
            if relation_ids.insert(id.clone()) {
                break id;
            }
        };
        let (ids, start, end, list, relation_type) = if kind == "sldId" {
            (&mut slides, 256, 2147483647, "sldIdLst", "slide")
        } else {
            (
                &mut masters,
                2147483648,
                u32::MAX,
                "sldMasterIdLst",
                "slideMaster",
            )
        };
        let id = (start..=end)
            .find(|id| !ids.contains(id))
            .ok_or(PptxError::Limit("appended native IDs"))?;
        ids.insert(id);
        let child = format!(
            "<p:{kind} xmlns:p=\"{P}\" xmlns:r=\"{R}\" id=\"{id}\" r:id=\"{relation_id}\"/>"
        );
        xml = append_item(&xml, list, &child, limits, check)?;
        main_relations.push(Relationship::new(
            &owner,
            relation_id,
            format!("{R}/{relation_type}"),
            relocated(part)?.to_string(),
            false,
        )?);
    }
    relationships.insert(owner, main_relations);
    let mut rewrite = RewritePlan::new();
    rewrite.replace_part(main, xml)?;
    rewrite.append_graph(source, additions, relationships, check)?;
    Ok(rewrite.write_sealed(source, Vec::new(), check)?)
}

fn append_item(
    input: &[u8],
    list: &str,
    child: &str,
    limits: PackageLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, PptxError> {
    let mut ordinal = 0;
    let mut parent = None;
    let mut next = None;
    mo_xml::scan_with_control(input, limits.xml, check, |event| {
        if let XmlEvent::Start { element, depth, .. } = event {
            if depth == 1 && element.name.is(P, list) && parent.replace(ordinal).is_some() {
                return Err(mo_xml::XmlError::Malformed(
                    "duplicate native page list".into(),
                ));
            }
            // CT_Presentation orders masters before notes/handout masters and
            // slides, and slides before the required slide-size declaration.
            if depth == 1
                && next.is_none()
                && element.name.namespace == P
                && (list == "sldMasterIdLst" || element.name.local == "sldSz")
            {
                next = Some((ordinal, element.name.clone()));
            }
            ordinal += 1;
        }
        Ok(())
    })?;
    if let Some(parent) = parent {
        Ok(mo_xml::append_child(
            input,
            parent,
            &ExpandedName {
                namespace: P.into(),
                local: list.into(),
            },
            child,
            limits.xml,
            check,
        )?)
    } else {
        let (ordinal, name) =
            next.ok_or_else(|| conflict("source presentation has no size declaration"))?;
        Ok(mo_xml::insert_before(
            input,
            ordinal,
            &name,
            &format!("<p:{list} xmlns:p=\"{P}\">{child}</p:{list}>"),
            limits.xml,
            check,
        )?)
    }
}
fn conflict(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}
