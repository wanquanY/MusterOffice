use mo_opc::*;
use std::{cell::Cell, io::Write};

const TYPES: &[u8] = br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="bin" ContentType="application/octet-stream"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#;
const ROOT_RELS: &[u8] = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="urn:example:main" Target="doc/main.xml"/></Relationships>"#;
const ORIGINAL: &[u8] = br#"<?xml version="1.0"?><x:document xmlns:x="urn:example" xmlns:future="urn:future" future:keep="yes"><!--retain me--><future:payload a="&quot;opaque&quot;"/></x:document>"#;

fn name(value: &str) -> PartName {
    PartName::new(value).unwrap()
}
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = rawzip::ZipArchiveWriter::new(Vec::new());
    for (name, bytes) in entries {
        let (entry, config) = writer
            .new_file(rawzip::path::EntryPath::verbatim(name.as_bytes()))
            .start()
            .unwrap();
        let mut stream = config.wrap(entry);
        stream.write_all(bytes).unwrap();
        let (entry, descriptor) = stream.finish().unwrap();
        entry.finish(descriptor).unwrap();
    }
    writer.finish().unwrap()
}
fn fixture() -> Vec<u8> {
    zip(&[
        ("[Content_Types].xml", TYPES),
        ("_rels/.rels", ROOT_RELS),
        ("doc/main.xml", ORIGINAL),
        ("media/unknown.bin", b"\0opaque\xffresource"),
    ])
}
fn open(bytes: &[u8]) -> Result<Package<&[u8]>, OpcError> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
}

#[test]
fn streamed_open_resolves_graph_and_retains_opaque_bytes() {
    let bytes = fixture();
    let package = open(&bytes).unwrap();
    assert_eq!(package.parts().len(), 3);
    assert_eq!(
        package
            .read_part(&name("/DOC/main.xml"), 4096, &|| false)
            .unwrap(),
        ORIGINAL
    );
    let root = &package.relationships()[&RelationshipSource::Package][0];
    assert_eq!(
        root.resolved,
        RelationshipTarget::Internal {
            part: name("/doc/main.xml"),
            fragment: None
        }
    );
    assert_eq!(package.parts()[&name("/media/unknown.bin")].byte_length, 16);
    let mut copy = Vec::new();
    package
        .copy_part(&name("/media/unknown.bin"), &mut copy, &|| false)
        .unwrap();
    assert_eq!(copy, b"\0opaque\xffresource");
}

#[test]
fn no_op_rewrite_is_byte_identical_and_mutation_preserves_unedited_parts() {
    let original = fixture();
    let source = open(&original).unwrap();
    assert_eq!(
        RewritePlan::new().to_bytes(&source, &|| false).unwrap(),
        original
    );
    let mut plan = RewritePlan::new();
    plan.replace_part(name("/doc/main.xml"), b"<updated/>".to_vec())
        .unwrap();
    let result = plan.to_bytes(&source, &|| false).unwrap();
    let reopened = open(&result).unwrap();
    assert_eq!(
        reopened
            .read_part(&name("/doc/main.xml"), 100, &|| false)
            .unwrap(),
        b"<updated/>"
    );
    assert_eq!(
        reopened.parts()[&name("/media/unknown.bin")].sha256,
        source.parts()[&name("/media/unknown.bin")].sha256
    );
    assert_eq!(reopened.relationships(), source.relationships());
    assert_ne!(reopened.sha256(), source.sha256());
}

#[test]
fn equivalent_names_traversal_and_header_disagreement_are_rejected() {
    for invalid in [
        "../evil.xml",
        "/absolute.xml",
        "doc\\evil.xml",
        "doc//evil.xml",
        "doc/%2e%2e/evil.xml",
        "doc/evil.",
        "doc/%2Fetc.xml",
    ] {
        assert!(
            open(&zip(&[("[Content_Types].xml", TYPES), (invalid, b"<r/>")])).is_err(),
            "accepted {invalid}"
        );
    }
    assert!(
        open(&zip(&[
            ("[Content_Types].xml", TYPES),
            ("doc/a.xml", b"<r/>"),
            ("DOC/A.XML", b"<r/>")
        ]))
        .is_err()
    );
    assert!(
        open(&zip(&[
            ("[Content_Types].xml", TYPES),
            ("doc/a.xml", b"<r/>"),
            ("doc/a.xml", b"<r/>")
        ]))
        .is_err()
    );
    let mut bytes = fixture();
    bytes[30] = b'X'; // Change the first local name only, leaving its central record intact.
    assert!(open(&bytes).is_err());
}

#[test]
fn missing_targets_duplicate_relationship_ids_and_orphan_overrides_fail() {
    let missing = ROOT_RELS.to_vec();
    assert!(
        open(&zip(&[
            ("[Content_Types].xml", TYPES),
            ("_rels/.rels", &missing)
        ]))
        .is_err()
    );
    let duplicate = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="same" Type="urn:a" Target="doc/main.xml"/><Relationship Id="same" Type="urn:b" Target="doc/main.xml"/></Relationships>"#;
    assert!(
        open(&zip(&[
            ("[Content_Types].xml", TYPES),
            ("_rels/.rels", duplicate),
            ("doc/main.xml", ORIGINAL)
        ]))
        .is_err()
    );
    let orphan = br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/absent.xml" ContentType="application/xml"/></Types>"#;
    assert!(open(&zip(&[("[Content_Types].xml", orphan)])).is_err());
}

#[test]
fn crc_truncation_extra_bytes_and_entry_count_mismatches_are_rejected() {
    let original = fixture();
    let mut corrupt = original.clone();
    let offset = corrupt
        .windows(ORIGINAL.len())
        .position(|w| w == ORIGINAL)
        .unwrap();
    corrupt[offset + 10] ^= 1;
    assert!(open(&corrupt).is_err());
    assert!(open(&original[..original.len() - 2]).is_err());
    let mut appended = original.clone();
    appended.extend_from_slice(b"trailing");
    assert!(open(&appended).is_err());
    let mut count = original.clone();
    let eocd = count.len() - 22;
    count[eocd + 8..eocd + 12].fill(0);
    assert!(open(&count).is_err());
}

#[test]
fn deterministic_builder_uses_real_reopened_output_and_external_links_do_not_fetch() {
    let mut builder = PackageBuilder::new();
    builder
        .add_part(name("/doc/main.xml"), "application/xml".into(), ORIGINAL)
        .unwrap();
    builder
        .set_relationships(
            RelationshipSource::Package,
            vec![
                Relationship::new(
                    &RelationshipSource::Package,
                    "root".into(),
                    "urn:example:main".into(),
                    "doc/main.xml".into(),
                    false,
                )
                .unwrap(),
            ],
        )
        .unwrap();
    let source = RelationshipSource::Part(name("/doc/main.xml"));
    builder
        .set_relationships(
            source.clone(),
            vec![
                Relationship::new(
                    &source,
                    "link".into(),
                    "urn:example:external".into(),
                    "https://example.invalid/no-network".into(),
                    true,
                )
                .unwrap(),
            ],
        )
        .unwrap();
    let first = builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    let second = builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    assert_eq!(first, second);
    let result = open(&first).unwrap();
    assert_eq!(
        result
            .read_part(&name("/doc/main.xml"), 4096, &|| false)
            .unwrap(),
        ORIGINAL
    );
    assert_eq!(
        result.relationships()[&source][0].resolved,
        RelationshipTarget::External
    );
}

#[test]
fn stream_resources_are_digest_bound_and_reads_stay_bounded() {
    struct Source {
        data: Vec<u8>,
        largest_read: Cell<usize>,
    }
    impl ReaderAt for Source {
        fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
            self.largest_read
                .set(self.largest_read.get().max(buf.len()));
            self.data.as_slice().read_at(buf, offset)
        }
    }
    let source = Source {
        data: (0..300_000).map(|n| (n % 251) as u8).collect(),
        largest_read: Cell::new(0),
    };
    let mut hash = sha2::Sha256::default();
    use sha2::Digest as _;
    hash.update(&source.data);
    let digest = mo_common::Digest::from_sha256(hash.finalize().into());
    let mut builder = PackageBuilder::new();
    builder
        .add_resource(
            name("/media/data.bin"),
            "application/octet-stream".into(),
            &source,
            source.data.len() as u64,
            digest.clone(),
        )
        .unwrap();
    let bytes = builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    let package = open(&bytes).unwrap();
    assert_eq!(package.parts()[&name("/media/data.bin")].sha256, digest);
    assert!(source.largest_read.get() <= 65_536);
    let mut invalid = PackageBuilder::new();
    invalid
        .add_resource(
            name("/media/data.bin"),
            "application/octet-stream".into(),
            &source,
            source.data.len() as u64,
            mo_common::Digest::from_sha256([0; 32]),
        )
        .unwrap();
    assert!(matches!(
        invalid.to_bytes(PackageLimits::default(), &|| false),
        Err(OpcError::Preservation(_))
    ));
}

#[test]
fn cancellation_and_declared_budgets_fail_before_unbounded_work() {
    let bytes = fixture();
    assert!(matches!(
        Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| true
        ),
        Err(OpcError::Cancelled)
    ));
    for limits in [
        PackageLimits {
            max_package_bytes: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_parts: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_inflated_bytes: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_part_bytes: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_relationships: 0,
            ..PackageLimits::default()
        },
    ] {
        assert!(Package::open(bytes.as_slice(), bytes.len() as u64, limits, &|| false).is_err());
    }
    let source = open(&bytes).unwrap();
    let mut output = Vec::new();
    assert!(matches!(
        RewritePlan::new().write_to(&source, &mut output, &|| true),
        Err(OpcError::Cancelled)
    ));
    assert!(output.is_empty());
}

#[test]
fn signed_packages_may_be_copied_but_not_silently_modified() {
    let mut builder = PackageBuilder::new();
    builder
        .add_part(name("/doc/main.xml"), "application/xml".into(), ORIGINAL)
        .unwrap();
    builder
        .add_part(
            name("/_xmlsignatures/sig1.xml"),
            "application/vnd.openxmlformats-package.digital-signature-xmlsignature+xml".into(),
            b"<signature/>".as_slice(),
        )
        .unwrap();
    let original = builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    let package = open(&original).unwrap();
    assert!(package.has_signatures());
    assert_eq!(
        RewritePlan::new().to_bytes(&package, &|| false).unwrap(),
        original
    );
    let mut plan = RewritePlan::new();
    plan.replace_part(name("/doc/main.xml"), b"<edited/>".to_vec())
        .unwrap();
    assert!(matches!(
        plan.to_bytes(&package, &|| false),
        Err(OpcError::Preservation(_))
    ));
}

#[test]
fn part_names_and_relationship_resolution_follow_uri_scope() {
    assert_eq!(name("/A/Slide.XML"), name("/a/slide.xml"));
    assert!(PartName::new("/a/%41.xml").is_err());
    let source = RelationshipSource::Part(name("/ppt/slides/slide1.xml"));
    let (resolved, fragment) =
        resolve_internal_target(&source, "../media/image1.svg#view").unwrap();
    assert_eq!(resolved, name("/ppt/media/image1.svg"));
    assert_eq!(fragment.as_deref(), Some("view"));
    assert_eq!(
        relationship_source(&relationship_part_name(&source).unwrap()).unwrap(),
        Some(source.clone())
    );
    for bad in [
        "../../../../outside.xml",
        "https://example.com/a.xml",
        "//example.com/a.xml",
        "../media/a.xml?query",
        "../%2Foutside.xml",
    ] {
        assert!(
            resolve_internal_target(&source, bad).is_err(),
            "accepted {bad}"
        );
    }
    assert!(relationship_source(&name("/ppt/_rels/_rels/a.xml.rels.rels")).is_err());
}

#[test]
fn malformed_xml_in_a_regular_part_is_not_hidden_by_a_valid_zip() {
    for xml in [
        b"<r>".as_slice(),
        b"<!DOCTYPE r SYSTEM 'https://example.invalid'><r/>",
    ] {
        assert!(open(&zip(&[("[Content_Types].xml", TYPES), ("doc/a.xml", xml)])).is_err());
    }
}

#[test]
fn writer_validates_xml_bytes_and_resources_before_exposing_output() {
    let bad = b"<unclosed>".as_slice();
    for as_resource in [false, true] {
        let mut builder = PackageBuilder::new();
        if as_resource {
            use sha2::Digest as _;
            let hash = mo_common::Digest::from_sha256(sha2::Sha256::digest(bad).into());
            builder
                .add_resource(
                    name("/doc/a.xml"),
                    "application/xml".into(),
                    &bad,
                    bad.len() as u64,
                    hash,
                )
                .unwrap();
        } else {
            builder
                .add_part(name("/doc/a.xml"), "application/xml".into(), bad)
                .unwrap();
        }
        let mut sink = Vec::new();
        assert!(matches!(
            builder.write_to(&mut sink, PackageLimits::default(), &|| false),
            Err(OpcError::Xml { .. })
        ));
        assert!(sink.is_empty());
    }
}

#[test]
fn duplicate_rewrite_does_not_replace_the_already_prepared_value() {
    let bytes = fixture();
    let source = open(&bytes).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(name("/doc/main.xml"), b"<first/>".to_vec())
        .unwrap();
    assert!(
        plan.replace_part(name("/doc/main.xml"), b"<second/>".to_vec())
            .is_err()
    );
    let result = plan.to_bytes(&source, &|| false).unwrap();
    assert_eq!(
        open(&result)
            .unwrap()
            .read_part(&name("/doc/main.xml"), 100, &|| false)
            .unwrap(),
        b"<first/>"
    );
}

#[test]
fn overlapping_local_entries_are_rejected_even_with_matching_names_and_crc() {
    let inner = zip(&[("doc/inner.xml", b"<r/>")]);
    let mut outer = zip(&[
        ("[Content_Types].xml", TYPES),
        ("blob.bin", &inner),
        ("doc/inner.xml", b"<r/>"),
    ]);
    let inner_start = outer.windows(inner.len()).position(|w| w == inner).unwrap();
    let eocd = outer.len() - 22;
    let directory = u32::from_le_bytes(outer[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let central = directory
        + outer[directory..]
            .windows(4)
            .rposition(|w| w == b"PK\x01\x02")
            .unwrap();
    outer[central + 42..central + 46].copy_from_slice(&(inner_start as u32).to_le_bytes());
    let error = open(&outer).err().unwrap().to_string();
    assert!(error.contains("overlapping"), "{error}");
}
