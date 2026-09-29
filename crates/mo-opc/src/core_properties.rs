//! OPC core properties are package metadata shared by Office document kinds.
//! Unknown properties are retained; only the explicitly projected title is read.
use crate::{
    OpcError, PackageRead, PartName, RelationshipSource, RelationshipTarget, check_cancel, metadata,
};
use mo_xml::{ExpandedName, XmlEvent, XmlLimits};

pub const CORE_PROPERTIES_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
pub const CORE_PROPERTIES_TYPE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
pub(crate) const CORE_NS: &str =
    "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
pub(crate) const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreProperties {
    pub part: PartName,
    pub title: String,
    pub(crate) title_ordinal: Option<usize>,
}
pub fn read_core_properties(
    source: &dyn PackageRead,
    limits: XmlLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<CoreProperties>, OpcError> {
    check_cancel(cancelled)?;
    let mut part = None;
    for relation in source
        .relationships()
        .get(&RelationshipSource::Package)
        .into_iter()
        .flatten()
    {
        if relation.relationship_type != CORE_PROPERTIES_RELATIONSHIP {
            continue;
        }
        let RelationshipTarget::Internal {
            part: target,
            fragment: None,
        } = &relation.resolved
        else {
            return Err(invalid("core properties must be an internal whole part"));
        };
        if part.replace(target.clone()).is_some() {
            return Err(invalid("multiple core properties relationships"));
        }
        if source.parts().get(target).map(|p| p.content_type.as_str()) != Some(CORE_PROPERTIES_TYPE)
        {
            return Err(invalid("core properties content type mismatch"));
        }
    }
    let count = source
        .parts()
        .values()
        .filter(|p| p.content_type == CORE_PROPERTIES_TYPE)
        .count();
    if count != usize::from(part.is_some()) {
        return Err(invalid("ambiguous or orphan core properties part"));
    }
    let Some(part) = part else {
        return Ok(None);
    };
    let bytes = source.read_part(&part, limits.max_bytes as u64, cancelled)?;
    let mut title = String::new();
    let mut ordinal = 0usize;
    let mut title_ordinal = None;
    let mut in_title = false;
    mo_xml::scan_with_control(&bytes, limits, cancelled, |event| {
        match event {
            XmlEvent::Start { element, depth, .. } => {
                if depth == 0 && !element.name.is(CORE_NS, "coreProperties") {
                    return Err(mo_xml::XmlError::Malformed(
                        "core properties root mismatch".into(),
                    ));
                }
                if in_title {
                    return Err(mo_xml::XmlError::EditConflict(
                        "structured core title is not a text leaf".into(),
                    ));
                }
                if depth == 1 && element.name.is(DC_NS, "title") {
                    if title_ordinal.replace(ordinal).is_some() {
                        return Err(mo_xml::XmlError::Malformed("duplicate core title".into()));
                    }
                    in_title = true;
                }
                ordinal += 1;
            }
            XmlEvent::Text { text, .. } if in_title => title.push_str(text),
            XmlEvent::End { depth: 1, .. } => in_title = false,
            _ => {}
        }
        Ok(())
    })
    .map_err(|e| metadata::xml_error(part.to_string(), e))?;
    Ok(Some(CoreProperties {
        part,
        title,
        title_ordinal,
    }))
}
pub(crate) fn name(namespace: &str, local: &str) -> ExpandedName {
    ExpandedName {
        namespace: namespace.into(),
        local: local.into(),
    }
}
fn invalid(message: &str) -> OpcError {
    OpcError::Structure(message.into())
}
