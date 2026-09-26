use crate::{
    CONTENT_TYPES_NAME, ContentTypes, OpcError, PackageLimits, PartName, RELS_TYPE, Relationship,
    RelationshipSource, RelationshipTarget, check_cancel, metadata, relationship_source,
};
use mo_common::Digest;
use rawzip::{CompressionMethod, ReaderAt, ZipArchive, ZipArchiveEntryWayfinder};
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartInfo {
    pub name: PartName,
    pub content_type: String,
    pub byte_length: u64,
    pub compressed_length: u64,
    pub sha256: Digest,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexedEntry {
    pub name: String,
    pub wayfinder: ZipArchiveEntryWayfinder,
    pub compression: CompressionMethod,
    pub crc32: u32,
    pub byte_length: u64,
    pub compressed_length: u64,
}

/// ReaderAt is an immutable, authorized host resource, never an arbitrary pathname.
/// Decompression is streamed; media bytes are not retained in the graph.
pub struct Package<R> {
    pub(crate) archive: ZipArchive<R>,
    pub(crate) entries: BTreeMap<PartName, IndexedEntry>,
    pub(crate) types_entry: IndexedEntry,
    pub(crate) parts: BTreeMap<PartName, PartInfo>,
    pub(crate) content_types: ContentTypes,
    pub(crate) relationships: BTreeMap<RelationshipSource, Vec<Relationship>>,
    pub(crate) limits: PackageLimits,
    pub(crate) byte_length: u64,
    pub(crate) sha256: Digest,
}

impl<R: ReaderAt> Package<R> {
    pub fn open(
        reader: R,
        byte_length: u64,
        limits: PackageLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, OpcError> {
        check_cancel(cancelled)?;
        if byte_length > limits.max_package_bytes {
            return Err(OpcError::Limit("package bytes"));
        }
        let mut directory_buffer = vec![0; rawzip::MAX_CENTRAL_DIRECTORY_RECORD_SIZE];
        let archive = rawzip::ZipLocator::new()
            .max_search_space(65_557)
            .locate_in_reader(reader, &mut directory_buffer, byte_length)
            .map_err(|(_, e)| OpcError::Zip(e))?;
        if archive.end_offset() != byte_length {
            return Err(OpcError::Structure("bytes after ZIP end record".into()));
        }
        if archive.entries_hint() > (limits.max_parts as u64).saturating_add(1) {
            return Err(OpcError::Limit("ZIP entry count"));
        }
        let mut entries = BTreeMap::new();
        let mut types_entry = None;
        let mut names = BTreeSet::new();
        let mut ranges = Vec::new();
        let mut count = 0_u64;
        let mut total = 0_u64;
        let mut local_buffer = vec![0; 131_070];
        let mut iterator = archive.entries(&mut directory_buffer);
        while let Some(header) = iterator.next_entry()? {
            check_cancel(cancelled)?;
            count += 1;
            if count > (limits.max_parts as u64).saturating_add(1) {
                return Err(OpcError::Limit("ZIP entry count"));
            }
            let raw_name = header.file_path();
            let name = std::str::from_utf8(raw_name.as_ref())
                .map_err(|_| OpcError::Structure("invalid ZIP name UTF-8".into()))?;
            if !names.insert(name.to_ascii_lowercase()) {
                return Err(OpcError::Structure(
                    "duplicate or equivalent ZIP entry name".into(),
                ));
            }
            if header.flags().is_encrypted()
                || header.flags().has_strong_encryption()
                || header.flags().is_masked()
            {
                return Err(OpcError::Unsupported(
                    "ZIP encryption is not an OPC encryption envelope".into(),
                ));
            }
            if header.flags().bits() & !0x080E != 0 {
                return Err(OpcError::Unsupported("unsupported ZIP entry flags".into()));
            }
            let mode = header.mode().value() & 0o170000;
            if !matches!(mode, 0 | 0o100000 | 0o040000) {
                return Err(OpcError::Structure("non-regular ZIP entry".into()));
            }
            let compression = header.compression_method();
            if compression != CompressionMethod::STORE && compression != CompressionMethod::DEFLATE
            {
                return Err(OpcError::Unsupported(format!(
                    "OPC compression method {compression:?}"
                )));
            }
            let size = header.uncompressed_size_hint();
            let compressed = header.compressed_size_hint();
            if size > limits.max_part_bytes {
                return Err(OpcError::Limit("part bytes"));
            }
            total = total
                .checked_add(size)
                .ok_or(OpcError::Limit("total inflated bytes"))?;
            if total > limits.max_inflated_bytes {
                return Err(OpcError::Limit("total inflated bytes"));
            }
            if u128::from(size)
                > u128::from(compressed.max(1)) * u128::from(limits.max_compression_ratio)
            {
                return Err(OpcError::Limit("compression ratio"));
            }
            let local = archive.get_entry(header.wayfinder())?;
            let local_header = local.local_header(&mut local_buffer)?;
            if local_header.file_path().as_ref() != raw_name.as_ref()
                || local_header.flags() != header.flags()
                || local_header.compression_method() != compression
            {
                return Err(OpcError::Structure(
                    "central/local ZIP header disagreement".into(),
                ));
            }
            if !header.flags().has_data_descriptor()
                && (local_header.crc32() != header.crc32()
                    || local_header.uncompressed_size_hint() != size
                    || local_header.compressed_size_hint() != compressed)
            {
                return Err(OpcError::Structure(
                    "central/local ZIP size or CRC disagreement".into(),
                ));
            }
            let (_, end) = local.compressed_data_range();
            let end = crate::zip_structure::occupied_end(
                archive.get_ref(),
                &header,
                end,
                archive.directory_offset(),
            )?;
            ranges.push((header.local_header_offset(), end));
            if header.is_dir() {
                PartName::new(format!("/{}", name.strip_suffix('/').unwrap_or(name)))?;
                if size != 0 || header.crc32() != 0 {
                    return Err(OpcError::Structure("directory contains data".into()));
                }
                continue;
            }
            let entry = IndexedEntry {
                name: name.into(),
                wayfinder: header.wayfinder(),
                compression,
                crc32: header.crc32(),
                byte_length: size,
                compressed_length: compressed,
            };
            if name == CONTENT_TYPES_NAME {
                types_entry = Some(entry);
            } else {
                let part = PartName::new(format!("/{name}"))?;
                if entries.insert(part, entry).is_some() {
                    return Err(OpcError::Structure("equivalent part names".into()));
                }
            }
        }
        if count != archive.entries_hint() {
            return Err(OpcError::Structure(
                "ZIP entry count differs from end record".into(),
            ));
        }
        ranges.sort_unstable();
        if ranges.first().is_some_and(|(start, _)| *start != 0) {
            return Err(OpcError::Structure("OPC ZIP has a preamble".into()));
        }
        if ranges
            .windows(2)
            .any(|p| p[0].1 > p[1].0 || p[0].0 == p[1].0)
        {
            return Err(OpcError::Structure("overlapping ZIP entries".into()));
        }
        let types_entry =
            types_entry.ok_or_else(|| OpcError::Structure("missing [Content_Types].xml".into()))?;
        let types_bytes = collect_entry(
            &archive,
            &types_entry,
            limits.xml.max_bytes as u64,
            cancelled,
        )?;
        let content_types = ContentTypes::from_xml(&types_bytes, limits, cancelled)?;
        let mut parts = BTreeMap::new();
        let mut relationships = BTreeMap::new();
        let mut relationship_count = 0_usize;
        for (name, entry) in &entries {
            check_cancel(cancelled)?;
            let content_type = content_types
                .content_type(name)
                .ok_or_else(|| OpcError::Structure(format!("missing content type for {name}")))?
                .to_owned();
            let mut hash = Sha256::new();
            let xml = if metadata::is_xml(&content_type) {
                let bytes = collect_entry(&archive, entry, limits.xml.max_bytes as u64, cancelled)?;
                hash.update(&bytes);
                Some(bytes)
            } else {
                stream_entry(&archive, entry, limits.max_part_bytes, cancelled, |bytes| {
                    hash.update(bytes);
                    Ok(())
                })?;
                None
            };
            let sha256 = Digest::from_sha256(hash.finalize().into());
            if let Some(source) = relationship_source(name)? {
                if !content_type.eq_ignore_ascii_case(RELS_TYPE) {
                    return Err(OpcError::Structure(
                        "relationship part has wrong content type".into(),
                    ));
                }
                let values = metadata::parse_relationships(
                    &source,
                    xml.as_deref().expect("rels is an XML content type"),
                    limits,
                    cancelled,
                )?;
                relationship_count = relationship_count
                    .checked_add(values.len())
                    .ok_or(OpcError::Limit("relationship count"))?;
                if relationship_count > limits.max_relationships {
                    return Err(OpcError::Limit("relationship count"));
                }
                relationships.insert(source, values);
            } else if content_type.eq_ignore_ascii_case(RELS_TYPE) {
                return Err(OpcError::Structure(
                    "relationship content type at invalid location".into(),
                ));
            } else if let Some(bytes) = xml {
                mo_xml::scan_with_control(&bytes, limits.xml, cancelled, |_| Ok(())).map_err(
                    |source| {
                        if matches!(source, mo_xml::XmlError::Cancelled) {
                            OpcError::Cancelled
                        } else {
                            OpcError::Xml {
                                part: name.to_string(),
                                source,
                            }
                        }
                    },
                )?;
            }
            parts.insert(
                name.clone(),
                PartInfo {
                    name: name.clone(),
                    content_type,
                    byte_length: entry.byte_length,
                    compressed_length: entry.compressed_length,
                    sha256,
                },
            );
        }
        validate_graph(
            &parts.keys().cloned().collect(),
            &content_types,
            &relationships,
        )?;
        let mut hash = Sha256::new();
        stream_range(archive.get_ref(), 0, byte_length, cancelled, |bytes| {
            hash.update(bytes);
            Ok(())
        })?;
        Ok(Self {
            archive,
            entries,
            types_entry,
            parts,
            content_types,
            relationships,
            limits,
            byte_length,
            sha256: Digest::from_sha256(hash.finalize().into()),
        })
    }
    pub fn parts(&self) -> &BTreeMap<PartName, PartInfo> {
        &self.parts
    }
    pub fn relationships(&self) -> &BTreeMap<RelationshipSource, Vec<Relationship>> {
        &self.relationships
    }
    pub fn content_types(&self) -> &ContentTypes {
        &self.content_types
    }
    pub fn sha256(&self) -> &Digest {
        &self.sha256
    }
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }
    pub fn read_part(
        &self,
        name: &PartName,
        max_bytes: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, OpcError> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| OpcError::Structure(format!("part does not exist: {name}")))?;
        collect_entry(
            &self.archive,
            entry,
            max_bytes.min(self.limits.max_part_bytes),
            cancelled,
        )
    }
    pub fn copy_part(
        &self,
        name: &PartName,
        writer: &mut impl Write,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), OpcError> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| OpcError::Structure(format!("part does not exist: {name}")))?;
        stream_entry(
            &self.archive,
            entry,
            self.limits.max_part_bytes,
            cancelled,
            |bytes| Ok(writer.write_all(bytes)?),
        )
    }
    pub fn has_signatures(&self) -> bool {
        self.parts.values().any(|p| {
            p.content_type
                .to_ascii_lowercase()
                .starts_with("application/vnd.openxmlformats-package.digital-signature")
        }) || self.relationships.values().flatten().any(|r| {
            r.relationship_type.starts_with(
                "http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/",
            )
        })
    }
}

pub(crate) fn validate_graph(
    names: &BTreeSet<PartName>,
    types: &ContentTypes,
    relationships: &BTreeMap<RelationshipSource, Vec<Relationship>>,
) -> Result<(), OpcError> {
    for name in names {
        if types.content_type(name).is_none() {
            return Err(OpcError::Structure(format!(
                "missing content type for {name}"
            )));
        }
        let mut parent = name.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            if prefix.is_empty() {
                break;
            }
            if names.contains(&PartName::new(prefix)?) {
                return Err(OpcError::Structure(
                    "one part name is a directory prefix of another".into(),
                ));
            }
            parent = prefix;
        }
    }
    for name in types.overrides.keys() {
        if !names.contains(name) {
            return Err(OpcError::Structure(format!(
                "orphan content type override for {name}"
            )));
        }
    }
    for (source, values) in relationships {
        if let RelationshipSource::Part(part) = source
            && (!names.contains(part) || relationship_source(part)?.is_some())
        {
            return Err(OpcError::Structure(
                "missing or invalid relationship source".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for relationship in values {
            if !ids.insert(&relationship.id) {
                return Err(OpcError::Structure("duplicate relationship Id".into()));
            }
            let reparsed = Relationship::new(
                source,
                relationship.id.clone(),
                relationship.relationship_type.clone(),
                relationship.target.clone(),
                relationship.resolved == RelationshipTarget::External,
            )?;
            if reparsed != *relationship {
                return Err(OpcError::Structure(
                    "resolved relationship disagrees with target URI".into(),
                ));
            }
            if let RelationshipTarget::Internal { part, .. } = &relationship.resolved
                && (!names.contains(part) || relationship_source(part)?.is_some())
            {
                return Err(OpcError::Structure(format!(
                    "missing or forbidden internal target {part}"
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn stream_range(
    reader: &impl ReaderAt,
    start: u64,
    length: u64,
    cancelled: &dyn Fn() -> bool,
    mut visit: impl FnMut(&[u8]) -> Result<(), OpcError>,
) -> Result<(), OpcError> {
    let mut buffer = [0_u8; 65_536];
    let mut offset = 0_u64;
    while offset < length {
        check_cancel(cancelled)?;
        let n = (length - offset).min(buffer.len() as u64) as usize;
        reader.read_exact_at(
            &mut buffer[..n],
            start
                .checked_add(offset)
                .ok_or(OpcError::Limit("source offset"))?,
        )?;
        visit(&buffer[..n])?;
        offset += n as u64;
    }
    Ok(())
}

fn collect_entry<R: ReaderAt>(
    archive: &ZipArchive<R>,
    entry: &IndexedEntry,
    limit: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, OpcError> {
    if entry.byte_length > limit {
        return Err(OpcError::Limit("part collection bytes"));
    }
    let mut bytes = Vec::new();
    stream_entry(archive, entry, limit, cancelled, |chunk| {
        bytes.extend_from_slice(chunk);
        Ok(())
    })?;
    Ok(bytes)
}

pub(crate) fn stream_entry<R: ReaderAt>(
    archive: &ZipArchive<R>,
    entry: &IndexedEntry,
    limit: u64,
    cancelled: &dyn Fn() -> bool,
    mut visit: impl FnMut(&[u8]) -> Result<(), OpcError>,
) -> Result<(), OpcError> {
    check_cancel(cancelled)?;
    let local = archive.get_entry(entry.wayfinder)?;
    let mut raw = local.reader();
    let mut inflated = flate2::read::DeflateDecoder::new(local.reader());
    let reader: &mut dyn Read = if entry.compression == CompressionMethod::STORE {
        &mut raw
    } else {
        &mut inflated
    };
    let mut reader = local.verifying_reader(reader);
    let mut buffer = [0_u8; 65_536];
    let mut total = 0_u64;
    loop {
        check_cancel(cancelled)?;
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(OpcError::Limit("inflated bytes"))?;
        if total > limit {
            return Err(OpcError::Limit("inflated bytes"));
        }
        visit(&buffer[..count])?;
    }
    if entry.compression == CompressionMethod::DEFLATE
        && inflated.total_in() != entry.compressed_length
    {
        return Err(OpcError::Structure(
            "trailing or truncated DEFLATE stream".into(),
        ));
    }
    Ok(())
}
