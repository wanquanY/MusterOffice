//! Add a typed graph closure without reserializing any existing metadata or
//! changing an existing relationship. Format semantics belong to the caller.
use super::*;
use crate::RelationshipTarget;

#[derive(Debug)]
pub struct GraphPart {
    pub name: PartName,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct GraphAppend {
    pub source: Digest,
    pub parts: BTreeMap<PartName, Vec<u8>>,
    pub types: Vec<u8>,
    pub added_types: BTreeMap<PartName, String>,
}

impl RewritePlan {
    pub fn append_graph<R: ReaderAt>(
        &mut self,
        source: &Package<R>,
        additions: Vec<GraphPart>,
        relationships: BTreeMap<RelationshipSource, Vec<Relationship>>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), OpcError> {
        check_cancel(cancelled)?;
        if self.graph_append.is_some() || self.core_title.is_some() {
            return Err(OpcError::Preservation(
                "graph extension must have one source baseline".into(),
            ));
        }
        let mut names: BTreeSet<_> = source.entries.keys().cloned().collect();
        let mut graph = source.relationships.clone();
        let mut types = source.content_types.clone();
        let mut parts = BTreeMap::new();
        let mut added_types = BTreeMap::new();
        for part in additions {
            check_cancel(cancelled)?;
            metadata::validate_content_type(&part.content_type)?;
            if relationship_source(&part.name)?.is_some()
                || part.content_type == RELS_TYPE
                || !names.insert(part.name.clone())
                || part.bytes.len() as u64 > source.limits.max_part_bytes
            {
                return Err(OpcError::Preservation(
                    "graph addition collides or exceeds capacity".into(),
                ));
            }
            types
                .overrides
                .insert(part.name.clone(), part.content_type.clone());
            added_types.insert(part.name.clone(), part.content_type);
            parts.insert(part.name, part.bytes);
        }
        for (owner, additions) in relationships {
            check_cancel(cancelled)?;
            if additions.is_empty() {
                continue;
            }
            let name = relationship_part_name(&owner)?;
            let current = graph.entry(owner.clone()).or_default();
            let mut ids: BTreeSet<_> = current.iter().map(|r| r.id.clone()).collect();
            let mut children = Vec::new();
            for relation in additions {
                let checked = Relationship::new(
                    &owner,
                    relation.id.clone(),
                    relation.relationship_type.clone(),
                    relation.target.clone(),
                    relation.resolved == RelationshipTarget::External,
                )?;
                if checked != relation || !ids.insert(relation.id.clone()) {
                    return Err(OpcError::Preservation(
                        "invalid or duplicate appended relationship".into(),
                    ));
                }
                children.push(format!(
                    "<Relationship xmlns=\"{}\" Id=\"{}\" Type=\"{}\" Target=\"{}\"{}/>",
                    metadata::RELS_NS,
                    metadata::escape(&relation.id)?,
                    metadata::escape(&relation.relationship_type)?,
                    metadata::escape(&relation.target)?,
                    if relation.resolved == RelationshipTarget::External {
                        " TargetMode=\"External\""
                    } else {
                        ""
                    }
                ));
                current.push(relation);
            }
            let bytes = if source.entries.contains_key(&name) {
                let mut bytes =
                    source.read_part(&name, source.limits.xml.max_bytes as u64, cancelled)?;
                for child in children {
                    bytes = append(
                        &bytes,
                        metadata::RELS_NS,
                        "Relationships",
                        &child,
                        source.limits,
                        cancelled,
                    )?;
                }
                bytes
            } else {
                names.insert(name.clone());
                types.overrides.insert(name.clone(), RELS_TYPE.into());
                added_types.insert(name.clone(), RELS_TYPE.into());
                metadata::serialize_relationships(current)?
            };
            if parts.insert(name, bytes).is_some() {
                return Err(OpcError::Preservation("duplicate graph part".into()));
            }
        }
        if names.len() > source.limits.max_parts
            || graph.values().map(Vec::len).sum::<usize>() > source.limits.max_relationships
        {
            return Err(OpcError::Limit("extended graph capacity"));
        }
        package::validate_graph(&names, &types, &graph)?;
        let mut types_xml = Vec::new();
        package::stream_entry(
            &source.archive,
            &source.types_entry,
            source.limits.xml.max_bytes as u64,
            cancelled,
            |bytes| {
                types_xml.extend_from_slice(bytes);
                Ok(())
            },
        )?;
        for (name, content_type) in &added_types {
            let child = format!(
                "<Override xmlns=\"{}\" PartName=\"{}\" ContentType=\"{}\"/>",
                metadata::TYPES_NS,
                metadata::escape(name.as_str())?,
                metadata::escape(content_type)?
            );
            types_xml = append(
                &types_xml,
                metadata::TYPES_NS,
                "Types",
                &child,
                source.limits,
                cancelled,
            )?;
        }
        self.graph_append = Some(GraphAppend {
            source: source.sha256.clone(),
            parts,
            types: types_xml,
            added_types,
        });
        Ok(())
    }
}
fn append(
    input: &[u8],
    namespace: &str,
    local: &str,
    child: &str,
    limits: PackageLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, OpcError> {
    mo_xml::append_child(
        input,
        0,
        &mo_xml::ExpandedName {
            namespace: namespace.into(),
            local: local.into(),
        },
        child,
        limits.xml,
        cancelled,
    )
    .map_err(|e| metadata::xml_error(local.into(), e))
}
