use mo_opc::*;
use std::io::Write;

const TYPES: &str = "<?xml version='1.0'?><Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Default Extension='xml' ContentType='application/xml'/><!--keep--><Override PartName='/absent.xml' ContentType='application/xml'></Override><Default Extension='bin' ContentType='application/octet-stream'/><Override PartName='/missing.xml' ContentType='application/xml'/></Types>";
const CONTENT: &[u8] = b"<doc xmlns='urn:owned'>editable</doc>";

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = rawzip::ZipArchiveWriter::new(Vec::new());
    for (name, bytes) in entries {
        let (entry, config) = zip
            .new_file(rawzip::path::EntryPath::verbatim(name.as_bytes()))
            .start()
            .unwrap();
        let mut stream = config.wrap(entry);
        stream.write_all(bytes).unwrap();
        let (entry, descriptor) = stream.finish().unwrap();
        entry.finish(descriptor).unwrap();
    }
    zip.finish().unwrap()
}
fn fixture() -> Vec<u8> {
    zip(&[
        ("[Content_Types].xml", TYPES.as_bytes()),
        ("doc.xml", CONTENT),
        ("image.bin", b"\x00\xffowned-media"),
    ])
}
fn inspect(bytes: &[u8]) -> Result<ContentTypeRepair<&[u8]>, OpcError> {
    ContentTypeRepair::inspect(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
}

#[test]
fn explicit_repair_preserves_all_real_parts_and_normal_open_stays_strict() {
    let source = fixture();
    assert!(
        Package::open(
            source.as_slice(),
            source.len() as u64,
            PackageLimits::default(),
            &|| false
        )
        .is_err()
    );
    let repair = inspect(&source).unwrap();
    assert_eq!(repair.removed_overrides().len(), 2);
    let output = repair
        .write_sealed(repair.source_sha256(), Vec::new(), &|| false)
        .unwrap();
    assert_eq!(output.package().parts().len(), 2);
    assert_eq!(
        output
            .package()
            .read_part(&PartName::new("/doc.xml").unwrap(), 1024, &|| false)
            .unwrap(),
        CONTENT
    );
    assert_eq!(
        output
            .package()
            .read_part(&PartName::new("/image.bin").unwrap(), 1024, &|| false)
            .unwrap(),
        b"\x00\xffowned-media"
    );
    let expected = TYPES
        .replace(
            "<Override PartName='/absent.xml' ContentType='application/xml'></Override>",
            "",
        )
        .replace(
            "<Override PartName='/missing.xml' ContentType='application/xml'/>",
            "",
        );
    // Decode the exact retained metadata stream independently of the repair API.
    let bytes = output.into_reader();
    let archive = rawzip::ZipArchive::from_slice(&bytes).unwrap();
    let mut entries = archive.entries();
    while let Some(header) = entries.next_entry().unwrap() {
        if header.file_path().as_ref() == b"[Content_Types].xml" {
            let entry = archive.get_entry(header.wayfinder()).unwrap();
            let mut actual = String::new();
            use std::io::Read;
            flate2::read::DeflateDecoder::new(entry.data())
                .read_to_string(&mut actual)
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
    let second = inspect(&bytes).unwrap();
    assert!(second.removed_overrides().is_empty());
    assert_eq!(
        second
            .write_sealed(second.source_sha256(), Vec::new(), &|| false)
            .unwrap()
            .into_reader(),
        bytes
    );
}

#[test]
fn repair_never_hides_missing_relationships_duplicate_names_or_bad_crc() {
    let rels=b"<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='r' Type='urn:owned' Target='missing.xml'/></Relationships>";
    let types=TYPES.replace("</Types>","<Default Extension='rels' ContentType='application/vnd.openxmlformats-package.relationships+xml'/></Types>");
    assert!(
        inspect(&zip(&[
            ("[Content_Types].xml", types.as_bytes()),
            ("_rels/.rels", rels),
            ("doc.xml", CONTENT)
        ]))
        .is_err()
    );
    assert!(
        inspect(&zip(&[
            ("[Content_Types].xml", TYPES.as_bytes()),
            ("doc.xml", CONTENT),
            ("DOC.XML", CONTENT)
        ]))
        .is_err()
    );
    let mut corrupt = fixture();
    let start = corrupt
        .windows(CONTENT.len())
        .position(|b| b == CONTENT)
        .unwrap();
    corrupt[start + 2] ^= 1;
    assert!(inspect(&corrupt).is_err());
}

#[test]
fn signed_input_cannot_be_repaired_and_source_identity_must_match() {
    let signature=TYPES.replace("</Types>","<Override PartName='/sig.xml' ContentType='application/vnd.openxmlformats-package.digital-signature-xmlsignature+xml'/></Types>");
    assert!(matches!(
        inspect(&zip(&[
            ("[Content_Types].xml", signature.as_bytes()),
            ("sig.xml", b"<signature/>"),
            ("doc.xml", CONTENT)
        ])),
        Err(OpcError::Preservation(_))
    ));
    let bytes = fixture();
    let repair = inspect(&bytes).unwrap();
    let wrong = mo_common::Digest::from_sha256([0; 32]);
    assert!(matches!(
        repair.write_sealed(&wrong, Vec::new(), &|| false),
        Err(OpcError::Preservation(_))
    ));
    assert!(matches!(
        repair.write_sealed(repair.source_sha256(), Vec::new(), &|| true),
        Err(OpcError::Cancelled)
    ));
    assert!(
        ContentTypeRepair::inspect(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits {
                max_package_bytes: 1,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
}
