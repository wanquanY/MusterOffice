//! Owned encoded images attached to an existing native package.
use mo_opc::{Package, PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource};
pub const PNG: &[u8] = include_bytes!("../../fixtures/presentations/resource-page/transparent.png");
pub const CYAN: &[u8] = include_bytes!("../../fixtures/presentations/resource-page/cyan.png");
pub fn with_image_resources(bytes: &[u8]) -> Vec<u8> {
    let source = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let mut builder = PackageBuilder::new();
    for (part, info) in source.parts() {
        if mo_opc::relationship_source(part).unwrap().is_some() {
            continue;
        }
        builder
            .add_part(
                part.clone(),
                info.content_type.clone(),
                source.read_part(part, 1 << 20, &|| false).unwrap(),
            )
            .unwrap();
    }
    for (n, bytes) in [
        ("owned-image", PNG),
        ("owned-copy", PNG),
        ("owned-cyan", CYAN),
    ] {
        builder
            .add_part(
                PartName::new(format!("/ppt/media/{n}.png")).unwrap(),
                "image/png".into(),
                bytes,
            )
            .unwrap();
    }
    for (part, relations) in source.relationships() {
        let mut relations = relations.clone();
        // Give every drawing surface its own declaring relationship owner so
        // master/layout inheritance and useBgFill can be probed independently.
        if matches!(part, RelationshipSource::Part(p) if p.as_str().ends_with(".xml")) {
            for id in ["owned-image", "owned-copy", "owned-cyan"] {
                relations.push(
                    Relationship::new(
                        part,
                        id.into(),
                        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                            .into(),
                        format!("/ppt/media/{id}.png"),
                        false,
                    )
                    .unwrap(),
                );
            }
            relations.push(
                Relationship::new(
                    part,
                    "owned-external".into(),
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                        .into(),
                    "https://example.invalid/explicit-image.png".into(),
                    true,
                )
                .unwrap(),
            );
        }
        builder.set_relationships(part.clone(), relations).unwrap();
    }
    builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap()
}
