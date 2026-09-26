//! Own synthetic corpus for independent ZIP verification. This is not a PPTX deck.
use mo_opc::{
    Package, PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource, RewritePlan,
};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("provide a fixture output directory")?,
    );
    fs::create_dir_all(&directory)?;
    let name = PartName::new("/doc/main.xml")?;
    let opaque = PartName::new("/future/opaque.bin")?;
    let mut builder = PackageBuilder::new();
    builder.add_part(name.clone(), "application/xml".into(), "<d xmlns=\"urn:fixture\" xmlns:f=\"urn:future\" f:keep=\"yes\"><!--opaque--><t>中文 🚀 é</t><f:node untouched=\"yes\"/></d>".as_bytes())?;
    builder.add_part(
        opaque,
        "application/octet-stream".into(),
        b"\0\xffopaque-future-data".as_slice(),
    )?;
    let source = RelationshipSource::Package;
    builder.set_relationships(
        source.clone(),
        vec![Relationship::new(
            &source,
            "rId1".into(),
            "urn:fixture:main".into(),
            "doc/main.xml".into(),
            false,
        )?],
    )?;
    let source = RelationshipSource::Part(name.clone());
    builder.set_relationships(
        source.clone(),
        vec![Relationship::new(
            &source,
            "rId2".into(),
            "urn:fixture:link".into(),
            "https://invalid.example/never-fetched".into(),
            true,
        )?],
    )?;
    let bytes = builder.to_bytes(PackageLimits::default(), &|| false)?;
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )?;
    fs::write(directory.join("kernel-built.opc"), &bytes)?;
    fs::write(
        directory.join("kernel-copy.opc"),
        RewritePlan::new().to_bytes(&package, &|| false)?,
    )?;
    let mut rewrite = RewritePlan::new();
    let original_xml = package.read_part(&name, 4096, &|| false)?;
    let edited_xml = mo_xml::rewrite_text(
        &original_xml,
        &[mo_xml::TextReplacement {
            element_ordinal: 1,
            expected_name: mo_xml::ExpandedName {
                namespace: "urn:fixture".into(),
                local: "t".into(),
            },
            expected_text: "中文 🚀 é".into(),
            replacement: "changed".into(),
        }],
        mo_xml::TextRewriteLimits::default(),
        &|| false,
    )?;
    rewrite.replace_part(name, edited_xml)?;
    fs::write(
        directory.join("kernel-edited.opc"),
        rewrite.to_bytes(&package, &|| false)?,
    )?;
    println!("created 3 synthetic OPC packages");
    Ok(())
}
