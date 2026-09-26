use crate::{
    CONTENT_TYPES_NAME, ContentTypes, OpcError, Package, PackageLimits, PartName, RELS_TYPE,
    ReaderAt, Relationship, RelationshipSource, ResultSink, VerifiedPackage, check_cancel,
    metadata,
    package::{self, IndexedEntry},
    relationship_part_name, relationship_source,
};
use mo_common::Digest;
use rawzip::{CompressionMethod, DataDescriptorOutput, ZipArchiveWriter};
use sha2::{Digest as _, Sha256};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

enum PartData<'a> {
    Bytes(Cow<'a, [u8]>),
    Resource {
        reader: &'a dyn ReaderAt,
        length: u64,
        sha256: Digest,
    },
}

struct NewPart<'a> {
    content_type: String,
    data: PartData<'a>,
}

/// The host supplies immutable bytes or authorized range-readable resources.
#[derive(Default)]
pub struct PackageBuilder<'a> {
    parts: BTreeMap<PartName, NewPart<'a>>,
    relationships: BTreeMap<RelationshipSource, Vec<Relationship>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReceipt {
    pub byte_length: u64,
    pub sha256: Digest,
}

impl<'a> PackageBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_part(
        &mut self,
        name: PartName,
        content_type: String,
        data: impl Into<Cow<'a, [u8]>>,
    ) -> Result<(), OpcError> {
        self.add(
            name,
            NewPart {
                content_type,
                data: PartData::Bytes(data.into()),
            },
        )
    }
    pub fn add_resource(
        &mut self,
        name: PartName,
        content_type: String,
        reader: &'a dyn ReaderAt,
        length: u64,
        sha256: Digest,
    ) -> Result<(), OpcError> {
        self.add(
            name,
            NewPart {
                content_type,
                data: PartData::Resource {
                    reader,
                    length,
                    sha256,
                },
            },
        )
    }
    fn add(&mut self, name: PartName, part: NewPart<'a>) -> Result<(), OpcError> {
        metadata::validate_content_type(&part.content_type)?;
        if relationship_source(&name)?.is_some()
            || part.content_type.eq_ignore_ascii_case(RELS_TYPE)
        {
            return Err(OpcError::Structure(
                "use set_relationships for relationship metadata".into(),
            ));
        }
        if self.parts.contains_key(&name) {
            return Err(OpcError::Structure("part already exists".into()));
        }
        self.parts.insert(name, part);
        Ok(())
    }
    pub fn set_relationships(
        &mut self,
        source: RelationshipSource,
        relationships: Vec<Relationship>,
    ) -> Result<(), OpcError> {
        relationship_part_name(&source)?;
        self.relationships.insert(source, relationships);
        Ok(())
    }
    pub fn write_to(
        &self,
        writer: impl Write,
        limits: PackageLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<WriteReceipt, OpcError> {
        check_cancel(cancelled)?;
        let mut types = ContentTypes::default();
        let mut names: BTreeSet<_> = self.parts.keys().cloned().collect();
        let mut relationship_bytes = BTreeMap::new();
        let mut total = 0_u64;
        for (name, part) in &self.parts {
            check_cancel(cancelled)?;
            let length = match &part.data {
                PartData::Bytes(b) => b.len() as u64,
                PartData::Resource { length, .. } => *length,
            };
            if length > limits.max_part_bytes {
                return Err(OpcError::Limit("part bytes"));
            }
            if metadata::is_xml(&part.content_type) {
                validate_xml_part(name, &part.data, limits, cancelled)?;
            }
            total = total
                .checked_add(length)
                .ok_or(OpcError::Limit("total inflated bytes"))?;
            types
                .overrides
                .insert(name.clone(), part.content_type.clone());
        }
        let mut relationship_count = 0_usize;
        for (source, relationships) in &self.relationships {
            let name = relationship_part_name(source)?;
            if !names.insert(name.clone()) {
                return Err(OpcError::Structure(
                    "relationship part name collides".into(),
                ));
            }
            relationship_count = relationship_count
                .checked_add(relationships.len())
                .ok_or(OpcError::Limit("relationship count"))?;
            let bytes = metadata::serialize_relationships(relationships)?;
            if bytes.len() > limits.xml.max_bytes {
                return Err(OpcError::Limit("relationship metadata bytes"));
            }
            total = total
                .checked_add(bytes.len() as u64)
                .ok_or(OpcError::Limit("total inflated bytes"))?;
            relationship_bytes.insert(name, bytes);
        }
        if !relationship_bytes.is_empty() {
            types.defaults.insert("rels".into(), RELS_TYPE.into());
        }
        if names.len() > limits.max_parts || relationship_count > limits.max_relationships {
            return Err(OpcError::Limit("part or relationship count"));
        }
        package::validate_graph(&names, &types, &self.relationships)?;
        let types_bytes = types.to_xml()?;
        total = total
            .checked_add(types_bytes.len() as u64)
            .ok_or(OpcError::Limit("total inflated bytes"))?;
        if total > limits.max_inflated_bytes || types_bytes.len() > limits.xml.max_bytes {
            return Err(OpcError::Limit("package metadata or inflated bytes"));
        }
        let mut zip = ZipArchiveWriter::new(Output::new(writer, limits.max_package_bytes));
        write_bytes(&mut zip, CONTENT_TYPES_NAME, &types_bytes, cancelled)?;
        for name in names {
            check_cancel(cancelled)?;
            if let Some(bytes) = relationship_bytes.get(&name) {
                write_bytes(&mut zip, name.zip_name(), bytes, cancelled)?;
            } else {
                match &self.parts[&name].data {
                    PartData::Bytes(bytes) => {
                        write_bytes(&mut zip, name.zip_name(), bytes, cancelled)?
                    }
                    PartData::Resource {
                        reader,
                        length,
                        sha256,
                    } => {
                        let (mut entry, config) = zip
                            .new_file(rawzip::path::EntryPath::verbatim(
                                name.zip_name().as_bytes(),
                            ))
                            .compression_method(CompressionMethod::DEFLATE)
                            .start()?;
                        let encoder = flate2::write::DeflateEncoder::new(
                            &mut entry,
                            flate2::Compression::new(6),
                        );
                        let mut stream = config.wrap(encoder);
                        let mut hash = Sha256::new();
                        package::stream_range(reader, 0, *length, cancelled, |bytes| {
                            hash.update(bytes);
                            Ok(stream.write_all(bytes)?)
                        })?;
                        if &Digest::from_sha256(hash.finalize().into()) != sha256 {
                            return Err(OpcError::Preservation(
                                "resource bytes differ from authorized content digest".into(),
                            ));
                        }
                        let (encoder, descriptor) = stream.finish()?;
                        encoder.finish()?;
                        entry.finish(descriptor)?;
                    }
                }
            }
        }
        check_cancel(cancelled)?;
        Ok(zip.finish()?.finish())
    }
    /// Streams into private host storage and verifies the actual sealed bytes.
    pub fn write_sealed<S: ResultSink>(
        &self,
        mut sink: S,
        limits: PackageLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VerifiedPackage<S::Reader>, OpcError> {
        let receipt = self.write_to(&mut sink, limits, cancelled)?;
        crate::sink::seal_and_verify(sink, receipt, limits, cancelled)
    }
    /// Convenience for small generated packages. Large hosts use write_sealed.
    pub fn to_bytes(
        &self,
        limits: PackageLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, OpcError> {
        Ok(self
            .write_sealed(Vec::new(), limits, cancelled)?
            .into_reader())
    }
}

/// Whole-part overlays for format adapters. Domain adapters remain responsible for
/// preserving unknown attributes/subtrees within a replaced part.
#[derive(Debug, Default)]
pub struct RewritePlan {
    replacements: BTreeMap<PartName, Vec<u8>>,
}

impl RewritePlan {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn replace_part(&mut self, part: PartName, bytes: Vec<u8>) -> Result<(), OpcError> {
        if relationship_source(&part)?.is_some() {
            return Err(OpcError::Preservation(
                "relationship changes require a typed graph rewrite".into(),
            ));
        }
        if self.replacements.contains_key(&part) {
            return Err(OpcError::Structure("duplicate replacement".into()));
        }
        self.replacements.insert(part, bytes);
        Ok(())
    }
    pub fn write_to<R: ReaderAt>(
        &self,
        source: &Package<R>,
        writer: impl Write,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<WriteReceipt, OpcError> {
        check_cancel(cancelled)?;
        if self.replacements.is_empty() {
            let mut output = Output::new(writer, source.limits.max_package_bytes);
            package::stream_range(
                source.archive.get_ref(),
                0,
                source.byte_length,
                cancelled,
                |bytes| Ok(output.write_all(bytes)?),
            )?;
            let receipt = output.finish();
            if receipt.sha256 != source.sha256 {
                return Err(OpcError::Preservation(
                    "source changed while copying".into(),
                ));
            }
            return Ok(receipt);
        }
        if source.has_signatures() {
            return Err(OpcError::Preservation(
                "signed package mutation needs an explicit remove/re-sign policy".into(),
            ));
        }
        let mut total = source.types_entry.byte_length;
        for name in self.replacements.keys() {
            if !source.entries.contains_key(name) {
                return Err(OpcError::Preservation(format!(
                    "replacement is not a source part: {name}"
                )));
            }
        }
        for (name, entry) in &source.entries {
            let size = self
                .replacements
                .get(name)
                .map_or(entry.byte_length, |b| b.len() as u64);
            if size > source.limits.max_part_bytes {
                return Err(OpcError::Limit("replacement part bytes"));
            }
            total = total
                .checked_add(size)
                .ok_or(OpcError::Limit("total inflated bytes"))?;
            if let Some(bytes) = self.replacements.get(name)
                && metadata::is_xml(&source.parts[name].content_type)
            {
                mo_xml::scan_with_control(bytes, source.limits.xml, cancelled, |_| Ok(()))
                    .map_err(|error| {
                        if cancelled() {
                            OpcError::Cancelled
                        } else {
                            OpcError::Xml {
                                part: name.to_string(),
                                source: error,
                            }
                        }
                    })?;
            }
        }
        if total > source.limits.max_inflated_bytes {
            return Err(OpcError::Limit("total inflated bytes"));
        }
        let mut zip = ZipArchiveWriter::new(Output::new(writer, source.limits.max_package_bytes));
        copy_compressed(&mut zip, source, &source.types_entry, cancelled)?;
        for (name, entry) in &source.entries {
            if let Some(bytes) = self.replacements.get(name) {
                write_bytes(&mut zip, &entry.name, bytes, cancelled)?;
            } else {
                copy_compressed(&mut zip, source, entry, cancelled)?;
            }
        }
        check_cancel(cancelled)?;
        Ok(zip.finish()?.finish())
    }
    pub fn write_sealed<R: ReaderAt, S: ResultSink>(
        &self,
        source: &Package<R>,
        mut sink: S,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VerifiedPackage<S::Reader>, OpcError> {
        let receipt = self.write_to(source, &mut sink, cancelled)?;
        crate::sink::seal_and_verify(sink, receipt, source.limits, cancelled)
    }
    pub fn to_bytes<R: ReaderAt>(
        &self,
        source: &Package<R>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, OpcError> {
        Ok(self
            .write_sealed(source, Vec::new(), cancelled)?
            .into_reader())
    }
}

fn validate_xml_part(
    name: &PartName,
    data: &PartData<'_>,
    limits: PackageLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), OpcError> {
    let bytes = match data {
        PartData::Bytes(bytes) => Cow::Borrowed(bytes.as_ref()),
        PartData::Resource {
            reader,
            length,
            sha256,
        } => {
            if *length > limits.xml.max_bytes as u64 {
                return Err(OpcError::Limit("XML resource bytes"));
            }
            let mut bytes = Vec::with_capacity(*length as usize);
            let mut hash = Sha256::new();
            package::stream_range(reader, 0, *length, cancelled, |chunk| {
                hash.update(chunk);
                bytes.extend_from_slice(chunk);
                Ok(())
            })?;
            if &Digest::from_sha256(hash.finalize().into()) != sha256 {
                return Err(OpcError::Preservation(
                    "XML resource differs from authorized content digest".into(),
                ));
            }
            Cow::Owned(bytes)
        }
    };
    mo_xml::scan_with_control(&bytes, limits.xml, cancelled, |_| Ok(()))
        .map_err(|error| metadata::xml_error(name.to_string(), error))?;
    Ok(())
}

fn copy_compressed<R: ReaderAt, W: Write>(
    zip: &mut ZipArchiveWriter<W>,
    source: &Package<R>,
    entry: &IndexedEntry,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), OpcError> {
    let original = source.archive.get_entry(entry.wayfinder)?;
    let (start, end) = original.compressed_data_range();
    let (mut target, _) = zip
        .new_file(rawzip::path::EntryPath::verbatim(entry.name.as_bytes()))
        .compression_method(entry.compression)
        .start()?;
    package::stream_range(
        source.archive.get_ref(),
        start,
        end - start,
        cancelled,
        |bytes| Ok(target.write_all(bytes)?),
    )?;
    target.finish(DataDescriptorOutput::new(entry.crc32, entry.byte_length))?;
    Ok(())
}

fn write_bytes<W: Write>(
    zip: &mut ZipArchiveWriter<W>,
    name: &str,
    bytes: &[u8],
    cancelled: &dyn Fn() -> bool,
) -> Result<(), OpcError> {
    let (mut entry, config) = zip
        .new_file(rawzip::path::EntryPath::verbatim(name.as_bytes()))
        .compression_method(CompressionMethod::DEFLATE)
        .start()?;
    let encoder = flate2::write::DeflateEncoder::new(&mut entry, flate2::Compression::new(6));
    let mut stream = config.wrap(encoder);
    for chunk in bytes.chunks(65_536) {
        check_cancel(cancelled)?;
        stream.write_all(chunk)?;
    }
    let (encoder, descriptor) = stream.finish()?;
    encoder.finish()?;
    entry.finish(descriptor)?;
    Ok(())
}

struct Output<W> {
    writer: W,
    hash: Sha256,
    written: u64,
    limit: u64,
}
impl<W: Write> Output<W> {
    fn new(writer: W, limit: u64) -> Self {
        Self {
            writer,
            hash: Sha256::new(),
            written: 0,
            limit,
        }
    }
    fn finish(self) -> WriteReceipt {
        WriteReceipt {
            byte_length: self.written,
            sha256: Digest::from_sha256(self.hash.finalize().into()),
        }
    }
}
impl<W: Write> Write for Output<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.limit.saturating_sub(self.written) {
            return Err(std::io::Error::other("output package byte budget exceeded"));
        }
        let written = self.writer.write(bytes)?;
        self.hash.update(&bytes[..written]);
        self.written += written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
