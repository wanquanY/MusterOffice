use crate::{OpcError, RelationshipSource};
use std::{cmp::Ordering, fmt};

/// Valid escaped OPC part URI. Equality is ASCII case-insensitive; spelling is preserved.
#[derive(Debug, Clone)]
pub struct PartName {
    spelling: String,
    key: String,
}

impl PartName {
    pub fn new(value: impl Into<String>) -> Result<Self, OpcError> {
        let spelling = value.into();
        let invalid = || OpcError::PartName(spelling.clone());
        if !spelling.starts_with('/')
            || spelling.len() < 2
            || spelling.len() > 65_535
            || !spelling.is_ascii()
        {
            return Err(invalid());
        }
        for segment in spelling[1..].split('/') {
            if segment.is_empty() || segment.ends_with('.') || segment.bytes().all(|b| b == b'.') {
                return Err(invalid());
            }
            let bytes = segment.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                let b = bytes[i];
                if b == b'%' {
                    let pair = bytes.get(i + 1..i + 3).ok_or_else(invalid)?;
                    let hex = |b: u8| (b as char).to_digit(16).map(|n| n as u8);
                    let decoded = hex(pair[0])
                        .zip(hex(pair[1]))
                        .map(|(h, l)| h * 16 + l)
                        .ok_or_else(invalid)?;
                    if unreserved(decoded)
                        || matches!(decoded, b'/' | b'\\')
                        || decoded < 0x20
                        || decoded == 0x7F
                    {
                        return Err(invalid());
                    }
                    i += 3;
                } else if unreserved(b) || b"!$&'()*+,;=:@".contains(&b) {
                    i += 1;
                } else {
                    return Err(invalid());
                }
            }
        }
        let key = spelling.to_ascii_lowercase();
        Ok(Self { spelling, key })
    }
    pub fn as_str(&self) -> &str {
        &self.spelling
    }
    pub fn zip_name(&self) -> &str {
        &self.spelling[1..]
    }
    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

fn unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"-._~".contains(&b)
}
impl PartialEq for PartName {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for PartName {}
impl PartialOrd for PartName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PartName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}
impl fmt::Display for PartName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.spelling)
    }
}

pub fn relationship_part_name(source: &RelationshipSource) -> Result<PartName, OpcError> {
    match source {
        RelationshipSource::Package => PartName::new("/_rels/.rels"),
        RelationshipSource::Part(source) => {
            if relationship_source(source)?.is_some() {
                return Err(OpcError::Structure(
                    "relationships cannot own relationships".into(),
                ));
            }
            let (folder, file) = source
                .as_str()
                .rsplit_once('/')
                .expect("validated part URI");
            PartName::new(format!("{folder}/_rels/{file}.rels"))
        }
    }
}

pub fn relationship_source(name: &PartName) -> Result<Option<RelationshipSource>, OpcError> {
    if name.key() == "/_rels/.rels" {
        return Ok(Some(RelationshipSource::Package));
    }
    let (folder, file) = name.as_str().rsplit_once('/').expect("validated part URI");
    if !folder.to_ascii_lowercase().ends_with("/_rels")
        || !file.to_ascii_lowercase().ends_with(".rels")
    {
        return Ok(None);
    }
    let parent = &folder[..folder.len() - 6];
    let file = &file[..file.len() - 5];
    if file.is_empty() {
        return Err(OpcError::Structure(
            "invalid relationship part location".into(),
        ));
    }
    let source = PartName::new(format!("{parent}/{file}"))?;
    if source.key() == "/_rels/.rels"
        || (parent.to_ascii_lowercase().ends_with("/_rels")
            && file.to_ascii_lowercase().ends_with(".rels"))
    {
        return Err(OpcError::Structure(
            "relationships cannot own relationships".into(),
        ));
    }
    Ok(Some(RelationshipSource::Part(source)))
}

pub fn resolve_internal_target(
    source: &RelationshipSource,
    target: &str,
) -> Result<(PartName, Option<String>), OpcError> {
    let (path, fragment) = target
        .split_once('#')
        .map_or((target, None), |(a, b)| (a, Some(b.to_owned())));
    if target.contains(['\\', '?'])
        || target.chars().any(|c| c.is_control() || c.is_whitespace())
        || path.starts_with("//")
    {
        return Err(OpcError::Structure(
            "invalid internal relationship target URI".into(),
        ));
    }
    if path.is_empty() {
        return match source {
            RelationshipSource::Part(part) => Ok((part.clone(), fragment)),
            _ => Err(OpcError::Structure(
                "package root is not a target part".into(),
            )),
        };
    }
    let mut segments: Vec<&str> = if path.starts_with('/') {
        Vec::new()
    } else {
        match source {
            RelationshipSource::Package => Vec::new(),
            RelationshipSource::Part(part) => part.as_str()[1..]
                .rsplit_once('/')
                .map(|(folder, _)| folder.split('/').collect())
                .unwrap_or_default(),
        }
    };
    if !path.starts_with('/') && path.split('/').next().is_some_and(|s| s.contains(':')) {
        return Err(OpcError::Structure(
            "absolute URI marked as internal relationship".into(),
        ));
    }
    for segment in path.strip_prefix('/').unwrap_or(path).split('/') {
        match segment {
            "." => (),
            ".." => {
                if segments.pop().is_none() {
                    return Err(OpcError::Structure(
                        "relationship escapes package root".into(),
                    ));
                }
            }
            "" => return Err(OpcError::Structure("empty target path segment".into())),
            _ => segments.push(segment),
        }
    }
    Ok((PartName::new(format!("/{}", segments.join("/")))?, fragment))
}
