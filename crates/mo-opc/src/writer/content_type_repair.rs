//! Deliberate, digest-bound repair of unused ContentType Override declarations.
//! Ordinary package import remains strict; this owner never exposes a partially
//! validated input as a usable Package. No part or relationship is removed.
use super::*;

pub struct ContentTypeRepair<R> {
    source: Package<R>,
    removed: BTreeMap<PartName, String>,
    types: Vec<u8>,
}

impl<R: ReaderAt> ContentTypeRepair<R> {
    pub fn inspect(
        reader: R,
        byte_length: u64,
        limits: PackageLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, OpcError> {
        let (source, removed) =
            Package::open_for_content_type_repair(reader, byte_length, limits, true, cancelled)?;
        if !removed.is_empty() && source.has_signatures() {
            return Err(OpcError::Preservation(
                "signed package repair needs an explicit remove/re-sign policy".into(),
            ));
        }
        let mut types = Vec::new();
        package::stream_entry(
            &source.archive,
            &source.types_entry,
            limits.xml.max_bytes as u64,
            cancelled,
            |b| {
                types.extend_from_slice(b);
                Ok(())
            },
        )?;
        let mut ordinal = 0;
        let mut removals = Vec::new();
        mo_xml::scan_with_control(&types, limits.xml, cancelled, |event| {
            if let mo_xml::XmlEvent::Start { element, depth, .. } = event {
                if depth == 1 && element.name.is(metadata::TYPES_NS, "Override") {
                    let name = PartName::new(element.attribute("PartName").unwrap_or_default())
                        .map_err(|_| {
                            mo_xml::XmlError::EditConflict("override name changed".into())
                        })?;
                    if removed.contains_key(&name) {
                        removals.push(mo_xml::ElementRemoval {
                            element_ordinal: ordinal,
                            expected_name: element.name.clone(),
                        });
                    }
                }
                ordinal += 1;
            }
            Ok(())
        })
        .map_err(|e| metadata::xml_error(CONTENT_TYPES_NAME.into(), e))?;
        types = mo_xml::remove_elements(&types, &removals, limits.xml, cancelled)
            .map_err(|e| metadata::xml_error(CONTENT_TYPES_NAME.into(), e))?;
        if ContentTypes::from_xml(&types, limits, cancelled)? != source.content_types {
            return Err(OpcError::Preservation(
                "repaired type declarations differ".into(),
            ));
        }
        Ok(Self {
            source,
            removed,
            types,
        })
    }

    pub fn source_sha256(&self) -> &Digest {
        self.source.sha256()
    }

    /// Every removed declaration is returned to the caller for an explicit audit.
    pub fn removed_overrides(&self) -> &BTreeMap<PartName, String> {
        &self.removed
    }

    /// The host supplies its reviewed source identity and private result sink.
    /// Success means actual output bytes passed normal strict OPC validation.
    pub fn write_sealed<S: ResultSink>(
        &self,
        expected_source: &Digest,
        mut sink: S,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VerifiedPackage<S::Reader>, OpcError> {
        check_cancel(cancelled)?;
        if expected_source != self.source.sha256() {
            return Err(OpcError::Preservation(
                "repair source digest changed".into(),
            ));
        }
        let receipt = if self.removed.is_empty() {
            RewritePlan::new().write_to(&self.source, &mut sink, cancelled)?
        } else {
            let mut zip =
                ZipArchiveWriter::new(Output::new(&mut sink, self.source.limits.max_package_bytes));
            write_bytes(&mut zip, CONTENT_TYPES_NAME, &self.types, cancelled)?;
            for entry in self.source.entries.values() {
                copy_compressed(&mut zip, &self.source, entry, cancelled)?;
            }
            check_cancel(cancelled)?;
            zip.finish()?.finish()
        };
        let verified = crate::sink::seal_and_verify(sink, receipt, self.source.limits, cancelled)?;
        if verified.package().parts() != self.source.parts()
            || verified.package().relationships() != self.source.relationships()
            || verified.package().content_types() != self.source.content_types()
        {
            return Err(OpcError::Preservation(
                "repair changed package content".into(),
            ));
        }
        Ok(verified)
    }
}
