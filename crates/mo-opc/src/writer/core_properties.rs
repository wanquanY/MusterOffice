//! Typed, source-pinned metadata updates. Relationship/content-type additions
//! are constructed here, never accepted as arbitrary part replacements.
use super::*;
use crate::core_properties::*;

#[derive(Debug)]
pub(super) struct CoreTitleEdit {
    pub source: Digest,
    pub title: String,
    pub parts: BTreeMap<PartName, Vec<u8>>,
    pub types: Option<Vec<u8>>,
}
impl RewritePlan {
    pub fn set_core_title<R: ReaderAt>(
        &mut self,
        source: &Package<R>,
        title: &str,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), OpcError> {
        check_cancel(cancelled)?;
        if self.core_title.is_some() || self.graph_append.is_some() {
            return Err(OpcError::Structure("duplicate core title edit".into()));
        }
        let original = read_core_properties(source, source.limits.xml, cancelled)?;
        if original.as_ref().map_or("", |p| p.title.as_str()) == title {
            return Ok(());
        }
        let leaf = title_leaf(title, source.limits.xml, cancelled)?;
        let mut parts = BTreeMap::new();
        let mut types = None;
        if let Some(original) = original {
            let input = source.read_part(
                &original.part,
                source.limits.xml.max_bytes as u64,
                cancelled,
            )?;
            let bytes = if let Some(element_ordinal) = original.title_ordinal {
                mo_xml::rewrite_text(
                    &input,
                    &[mo_xml::TextReplacement {
                        element_ordinal,
                        expected_name: name(DC_NS, "title"),
                        expected_text: original.title,
                        replacement: title.into(),
                    }],
                    mo_xml::TextRewriteLimits {
                        xml: source.limits.xml,
                        ..Default::default()
                    },
                    cancelled,
                )
            } else {
                mo_xml::append_child(
                    &input,
                    0,
                    &name(CORE_NS, "coreProperties"),
                    &leaf,
                    source.limits.xml,
                    cancelled,
                )
            }
            .map_err(|e| metadata::xml_error(original.part.to_string(), e))?;
            parts.insert(original.part, bytes);
        } else {
            let mut graph = source.relationships.clone();
            let mut content_types = source.content_types.clone();
            let mut names: BTreeSet<_> = source.entries.keys().cloned().collect();
            let core = unique_core_part(&names, source.limits.max_parts)?;
            let body =
                format!("<cp:coreProperties xmlns:cp=\"{CORE_NS}\">{leaf}</cp:coreProperties>")
                    .into_bytes();
            parts.insert(core.clone(), body);
            content_types
                .overrides
                .insert(core.clone(), CORE_PROPERTIES_TYPE.into());
            names.insert(core.clone());
            let relationships = graph.entry(RelationshipSource::Package).or_default();
            let id = (1..=source.limits.max_relationships)
                .map(|i| format!("moCore{i}"))
                .find(|id| relationships.iter().all(|r| r.id != *id))
                .ok_or(OpcError::Limit("core relationship identity"))?;
            let relation = Relationship::new(
                &RelationshipSource::Package,
                id,
                CORE_PROPERTIES_RELATIONSHIP.into(),
                core.to_string(),
                false,
            )?;
            let root = relationship_part_name(&RelationshipSource::Package)?;
            let relation_xml = format!(
                "<Relationship xmlns=\"{}\" Id=\"{}\" Type=\"{CORE_PROPERTIES_RELATIONSHIP}\" Target=\"{}\"/>",
                metadata::RELS_NS,
                relation.id,
                core
            );
            let root_xml = if source.entries.contains_key(&root) {
                let bytes =
                    source.read_part(&root, source.limits.xml.max_bytes as u64, cancelled)?;
                mo_xml::append_child(
                    &bytes,
                    0,
                    &name(metadata::RELS_NS, "Relationships"),
                    &relation_xml,
                    source.limits.xml,
                    cancelled,
                )
                .map_err(|e| metadata::xml_error(root.to_string(), e))?
            } else {
                content_types
                    .overrides
                    .insert(root.clone(), RELS_TYPE.into());
                metadata::serialize_relationships(std::slice::from_ref(&relation))?
            };
            relationships.push(relation);
            names.insert(root.clone());
            parts.insert(root, root_xml);
            if names.len() > source.limits.max_parts
                || graph.values().map(Vec::len).sum::<usize>() > source.limits.max_relationships
            {
                return Err(OpcError::Limit("core properties graph capacity"));
            }
            package::validate_graph(&names, &content_types, &graph)?;
            // Adding typed OPC declarations leaves original metadata spelling,
            // comments and encoding intact. Only new override children appear.
            let mut input = Vec::new();
            package::stream_entry(
                &source.archive,
                &source.types_entry,
                source.limits.xml.max_bytes as u64,
                cancelled,
                |b| {
                    input.extend_from_slice(b);
                    Ok(())
                },
            )?;
            for (part, media) in &content_types.overrides {
                if source.content_types.overrides.get(part) == Some(media) {
                    continue;
                }
                let child = format!(
                    "<Override xmlns=\"{}\" PartName=\"{part}\" ContentType=\"{media}\"/>",
                    metadata::TYPES_NS
                );
                input = mo_xml::append_child(
                    &input,
                    0,
                    &name(metadata::TYPES_NS, "Types"),
                    &child,
                    source.limits.xml,
                    cancelled,
                )
                .map_err(|e| metadata::xml_error(CONTENT_TYPES_NAME.into(), e))?;
            }
            types = Some(input);
        }
        self.core_title = Some(CoreTitleEdit {
            source: source.sha256.clone(),
            title: title.into(),
            parts,
            types,
        });
        Ok(())
    }
}
fn title_leaf(
    title: &str,
    limits: mo_xml::XmlLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<String, OpcError> {
    let input = format!("<dc:title xmlns:dc=\"{DC_NS}\"/>");
    let bytes = mo_xml::rewrite_text(
        input.as_bytes(),
        &[mo_xml::TextReplacement {
            element_ordinal: 0,
            expected_name: name(DC_NS, "title"),
            expected_text: String::new(),
            replacement: title.into(),
        }],
        mo_xml::TextRewriteLimits {
            xml: limits,
            ..Default::default()
        },
        cancelled,
    )
    .map_err(|e| metadata::xml_error("core title".into(), e))?;
    String::from_utf8(bytes).map_err(|_| OpcError::Structure("core title UTF-8".into()))
}
fn unique_core_part(names: &BTreeSet<PartName>, max: usize) -> Result<PartName, OpcError> {
    let root = if names.contains(&PartName::new("/docProps")?) {
        "/mo-core-properties"
    } else {
        "/docProps/core"
    };
    for i in 0..=max {
        let part = PartName::new(if i == 0 {
            format!("{root}.xml")
        } else {
            format!("{root}-{i}.xml")
        })?;
        let prefix = format!("{part}/");
        if !names.contains(&part) && !names.iter().any(|p| p.as_str().starts_with(&prefix)) {
            return Ok(part);
        }
    }
    Err(OpcError::Limit("core part identity"))
}
