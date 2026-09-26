#[path = "support/images.rs"]
mod fixtures;
mod support;
use fixtures::*;
use mo_common::{ByteLength, Digest};
use mo_opc::ReaderAt;
use mo_pptx::{
    PptxError,
    source::{SourceLimits, fill::resolve::FillTarget, images::*, inspect_source},
};
use serde_json::json;
use std::cell::Cell;

fn shape_request(bytes: &[u8], selection: ImageSourceSelection) -> SourceImageQuery {
    let i = inspect(bytes);
    request(
        &i,
        vec![FillTarget::Object {
            native_id: object(&i, SLIDE, "title:1"),
        }],
        selection,
    )
}
fn available(r: &SourceImageResources, i: usize) -> (u32, &SourceImageBinding) {
    match &r.targets[i].outcome {
        SourceImageOutcome::Available { resource, binding } => (*resource, binding),
        other => panic!("expected available: {other:?}"),
    }
}
#[test]
fn native_picture_extracts_original_bytes_and_reuses_canonical_part() {
    let mut f = Fixture::new();
    let b = base();
    let p = package(&b);
    let part = p
        .parts()
        .values()
        .find(|p| p.content_type == "image/png")
        .unwrap()
        .name
        .to_string();
    f.relation(SLIDE, "imageShared", "image", &part, false);
    f.shape(
        SLIDE,
        "title:1",
        &blip("r:embed=\"imageShared\""),
        false,
        false,
    );
    let b = f.finish();
    let i = inspect(&b);
    let q = request(
        &i,
        vec![
            FillTarget::Picture {
                native_id: object(&i, SLIDE, "picture:1"),
            },
            FillTarget::Object {
                native_id: object(&i, SLIDE, "title:1"),
            },
        ],
        ImageSourceSelection::EmbeddedSnapshot,
    );
    let r = execute("native-picture-reuse", &b, &q);
    assert_eq!(r.resources.len(), 1);
    assert_eq!(available(&r, 0).0, available(&r, 1).0);
    assert_eq!(r.resources[0].part, part);
    assert_eq!(
        extract(&package(&b), &r, SourceImageLimits::default(), &|| false).unwrap(),
        support::resources().0
    );
}
#[test]
fn identical_payloads_in_different_parts_keep_first_use_order_and_offsets() {
    let mut f = Fixture::new();
    let image = support::resources();
    f.part("/ppt/media/first.png", "image/png", image.0);
    f.part("/ppt/media/second.png", "image/png", image.0);
    f.relation(SLIDE, "first", "image", "../media/first.png", false);
    f.relation(SLIDE, "second", "image", "../media/second.png", false);
    f.shape(SLIDE, "title:1", &blip("r:embed=\"second\""), false, false);
    f.background(SLIDE, &blip("r:embed=\"first\""));
    let b = f.finish();
    let mut q = shape_request(&b, ImageSourceSelection::EmbeddedSnapshot);
    q.fill.targets.push(FillTarget::Background {});
    let r = execute("two-identical-parts", &b, &q);
    assert_eq!(r.resources.len(), 2);
    assert_eq!(r.resources[0].part, "/ppt/media/second.png");
    assert_eq!(r.resources[1].part, "/ppt/media/first.png");
    assert_eq!(r.resources[0].sha256, r.resources[1].sha256);
    assert_eq!(r.resources[1].offset.get(), image.0.len() as u64);
    assert_eq!(available(&r, 0).0, 0);
    assert_eq!(available(&r, 1).0, 1);
    assert_eq!(
        extract(&package(&b), &r, Default::default(), &|| false).unwrap(),
        [image.0, image.0].concat()
    );
}
#[test]
fn inherited_relationships_resolve_at_declaration_not_consuming_slide() {
    for (at, owner) in [SLIDE, LAYOUT, MASTER, THEME].into_iter().enumerate() {
        let mut f = Fixture::new();
        let shared = blip("r:embed=\"imageShared\"");
        for (n, (part, name)) in [
            (SLIDE, "title:1"),
            (LAYOUT, "rule:layout"),
            (MASTER, "footer:master"),
        ]
        .into_iter()
        .enumerate()
        {
            f.shape(
                part,
                name,
                if n == at { &shared } else { "" },
                true,
                at == 3 && n == 0,
            );
        }
        f.theme(if at == 3 { &shared } else { "<a:noFill/>" });
        for (n, part) in [SLIDE, LAYOUT, MASTER, THEME].into_iter().enumerate() {
            let target = format!("/ppt/media/owner{n}.svg");
            let svg =
                format!("<svg xmlns=\"http://www.w3.org/2000/svg\"><title>owner {n}</title></svg>");
            f.part(&target, "image/svg+xml", svg.as_bytes());
            f.relation(part, "imageShared", "image", &target, false);
        }
        let b = f.finish();
        let r = execute(
            &format!("owner-{at}"),
            &b,
            &shape_request(&b, ImageSourceSelection::EmbeddedSnapshot),
        );
        assert_eq!(available(&r, 0).1.reference.owner_part, owner);
        assert_eq!(r.resources[0].part, format!("/ppt/media/owner{at}.svg"));
    }
}
#[test]
fn embed_and_link_property_origins_and_selection_are_independent() {
    let mut f = Fixture::new();
    f.shape(
        SLIDE,
        "title:1",
        "<a:blipFill><a:tile tx=\"10\"/></a:blipFill>",
        true,
        true,
    );
    f.shape(
        LAYOUT,
        "rule:layout",
        &blip("r:link=\"shared\""),
        true,
        false,
    );
    f.shape(MASTER, "footer:master", "", true, false);
    f.theme(&blip("r:embed=\"shared\""));
    f.part(
        "/ppt/media/theme.svg",
        "image/svg+xml",
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    );
    f.relation(THEME, "shared", "image", "../media/theme.svg", false);
    f.relation(
        LAYOUT,
        "shared",
        "image",
        "https://example.invalid/linked.png",
        true,
    );
    f.relation(
        SLIDE,
        "shared",
        "image",
        "https://example.invalid/wrong.png",
        true,
    );
    let b = f.finish();
    let r = execute(
        "inherited-embedded",
        &b,
        &shape_request(&b, ImageSourceSelection::EmbeddedSnapshot),
    );
    let (_, binding) = available(&r, 0);
    assert_eq!(binding.reference.owner_part, THEME);
    assert_eq!(
        serde_json::to_value(binding).unwrap()["image"]["mode"]["kind"],
        "tile"
    );
    let r = execute(
        "inherited-linked",
        &b,
        &shape_request(&b, ImageSourceSelection::LinkedSource),
    );
    let SourceImageOutcome::ExternalRequired { binding } = &r.targets[0].outcome else {
        panic!("{r:?}")
    };
    assert_eq!(binding.reference.owner_part, LAYOUT);
    assert_eq!(
        binding.reference.target_uri,
        "https://example.invalid/linked.png"
    );
    assert!(r.resources.is_empty());
}
#[test]
fn source_selection_never_substitutes_other_slot() {
    for (name, attrs, selection) in [
        (
            "missing-embedded",
            "r:link=\"external\"",
            ImageSourceSelection::EmbeddedSnapshot,
        ),
        (
            "missing-linked",
            "r:embed=\"snapshot\"",
            ImageSourceSelection::LinkedSource,
        ),
    ] {
        let mut f = Fixture::new();
        f.shape(SLIDE, "title:1", &blip(attrs), false, false);
        f.part("/ppt/media/snapshot.svg", "image/svg+xml", b"<svg/>");
        f.relation(
            SLIDE,
            "external",
            "image",
            "https://example.invalid/x",
            true,
        );
        f.relation(SLIDE, "snapshot", "image", "../media/snapshot.svg", false);
        let b = f.finish();
        let r = execute(name, &b, &shape_request(&b, selection));
        assert!(
            matches!(r.targets[0].outcome,SourceImageOutcome::UnresolvedReference {ref issue} if matches!(**issue,ImageReferenceIssue::MissingSelectedReference {}))
        );
        assert!(r.resources.is_empty());
    }
}
#[test]
fn invalid_image_edges_are_explicit_and_never_publish_resource_bytes() {
    for (name, kind, target, external, attrs, selection) in [
        (
            "wrong-type",
            "hyperlink",
            "../media/x.svg",
            false,
            "r:embed=\"chosen\"",
            ImageSourceSelection::EmbeddedSnapshot,
        ),
        (
            "embed-external",
            "image",
            "https://example.invalid/x",
            true,
            "r:embed=\"chosen\"",
            ImageSourceSelection::EmbeddedSnapshot,
        ),
        (
            "link-internal",
            "image",
            "../media/x.svg",
            false,
            "r:link=\"chosen\"",
            ImageSourceSelection::LinkedSource,
        ),
        (
            "fragment",
            "image",
            "../media/x.svg#fragment",
            false,
            "r:embed=\"chosen\"",
            ImageSourceSelection::EmbeddedSnapshot,
        ),
        (
            "non-image",
            "image",
            "../media/x.dat",
            false,
            "r:embed=\"chosen\"",
            ImageSourceSelection::EmbeddedSnapshot,
        ),
        (
            "missing-relationship",
            "image",
            "../media/x.svg",
            false,
            "r:embed=\"absent\"",
            ImageSourceSelection::EmbeddedSnapshot,
        ),
    ] {
        let mut f = Fixture::new();
        f.part("/ppt/media/x.svg", "image/svg+xml", b"<svg/>");
        f.part("/ppt/media/x.dat", "application/octet-stream", b"opaque");
        f.shape(SLIDE, "title:1", &blip(attrs), false, false);
        f.relation(SLIDE, "chosen", kind, target, external);
        let b = f.finish();
        let r = execute(name, &b, &shape_request(&b, selection));
        let value = serde_json::to_value(&r.targets[0].outcome).unwrap();
        assert_eq!(value["status"], "unresolvedReference");
        assert_eq!(
            value["issue"]["kind"],
            match name {
                "wrong-type" => "wrongRelationshipType",
                "fragment" => "fragment",
                "non-image" => "nonImageContentType",
                "missing-relationship" => "missingRelationship",
                _ => "wrongTargetMode",
            }
        );
        assert!(r.resources.is_empty());
    }
}
#[test]
fn non_image_and_unsupported_effects_remain_distinct() {
    for (name, fill, status) in [
        (
            "solid",
            "<a:solidFill><a:srgbClr val=\"123456\"/></a:solidFill>",
            "notImage",
        ),
        (
            "image-effect",
            "<a:blipFill><a:blip r:embed=\"opaque\"><a:grayscl/></a:blip></a:blipFill>",
            "unresolvedFill",
        ),
    ] {
        let mut f = Fixture::new();
        f.shape(SLIDE, "title:1", fill, false, false);
        let b = f.finish();
        let r = execute(
            name,
            &b,
            &shape_request(&b, ImageSourceSelection::EmbeddedSnapshot),
        );
        assert_eq!(
            serde_json::to_value(&r.targets[0].outcome).unwrap()["status"],
            status
        );
        assert!(r.resources.is_empty());
    }
}
#[test]
fn group_and_background_bindings_keep_redirect_and_owner() {
    let mut f = Fixture::new();
    f.part("/ppt/media/x.svg", "image/svg+xml", b"<svg/>");
    for owner in [SLIDE, MASTER] {
        f.relation(owner, "chosen", "image", "../media/x.svg", false);
    }
    f.shape(SLIDE, "title:1", "<a:grpFill/>", false, false);
    f.xml(SLIDE,|mut x| {
        let a=x.find("<p:sp>").unwrap();let z=a+x[a..].find("</p:sp>").unwrap()+7;
        x.replace_range(a..z,&format!("<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"900\" name=\"images\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{}</p:grpSpPr>{}</p:grpSp>",blip("r:embed=\"chosen\""),&x[a..z]));x
    });
    let background = blip("r:embed=\"chosen\"");
    for owner in [SLIDE, LAYOUT, MASTER] {
        f.background(owner, if owner == MASTER { &background } else { "" });
    }
    let b = f.finish();
    let mut q = shape_request(&b, ImageSourceSelection::EmbeddedSnapshot);
    q.fill.targets.push(FillTarget::Background {});
    let r = execute("group-background", &b, &q);
    assert_eq!(r.resources.len(), 1);
    assert_eq!(available(&r, 0).1.reference.owner_part, SLIDE);
    assert_eq!(available(&r, 1).1.reference.owner_part, MASTER);
    assert!(!available(&r, 0).1.redirects.is_empty());
}
#[test]
fn image_query_limits_source_identity_and_cancellation_are_enforced() {
    let b = base();
    let p = package(&b);
    let i = inspect(&b);
    let q = request(
        &i,
        vec![FillTarget::Picture {
            native_id: object(&i, SLIDE, "picture:1"),
        }],
        ImageSourceSelection::EmbeddedSnapshot,
    );
    let good = query(&p, &i, &q, SourceImageLimits::default(), &|| false).unwrap();
    for limits in [
        SourceImageLimits {
            max_resources: 0,
            ..Default::default()
        },
        SourceImageLimits {
            max_relationship_steps: 0,
            ..Default::default()
        },
        SourceImageLimits {
            max_metadata_bytes: 0,
            ..Default::default()
        },
        SourceImageLimits {
            max_resource_bytes: 0,
            ..Default::default()
        },
        SourceImageLimits {
            max_bundle_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(&p, &i, &q, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    let mut wrong = q.clone();
    wrong.fill.expected_source_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(&p, &i, &wrong, SourceImageLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
    assert!(matches!(
        query(&p, &i, &q, SourceImageLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    assert!(matches!(
        extract(&p, &good, SourceImageLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    assert!(matches!(
        extract(
            &p,
            &good,
            SourceImageLimits {
                max_resource_bytes: 0,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
}
#[test]
fn extraction_validates_entire_catalog_before_reading_any_payload() {
    struct Counted<'a> {
        data: &'a [u8],
        reads: &'a Cell<usize>,
    }
    impl ReaderAt for Counted<'_> {
        fn read_at(&self, buffer: &mut [u8], offset: u64) -> std::io::Result<usize> {
            self.reads.set(self.reads.get() + 1);
            self.data.read_at(buffer, offset)
        }
    }
    let b = base();
    let reads = Cell::new(0);
    let p = mo_opc::Package::open(
        Counted {
            data: &b,
            reads: &reads,
        },
        b.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let i = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let q = request(
        &i,
        vec![FillTarget::Picture {
            native_id: object(&i, SLIDE, "picture:1"),
        }],
        ImageSourceSelection::EmbeddedSnapshot,
    );
    let good = query(&p, &i, &q, SourceImageLimits::default(), &|| false).unwrap();
    for mutation in 0..9 {
        let mut r = good.clone();
        match mutation {
            0 => r.resources[0].offset = ByteLength::new(1),
            1 => r.resources[0].sha256 = Digest::from_sha256([0; 32]),
            2 => r.resources[0].content_type = "image/jpeg".into(),
            3 => r.resources[0].byte_length = ByteLength::new(0),
            4 => r.resources[0].part = "/missing.png".into(),
            5 => r.bundle_byte_length = ByteLength::new(0),
            6 => r.resources.push(r.resources[0].clone()),
            7 => r.source_sha256 = Digest::from_sha256([0; 32]),
            _ => {
                let mut invalid = r.resources[0].clone();
                invalid.part = "/missing.png".into();
                r.resources.push(invalid);
            }
        }
        reads.set(0);
        assert!(extract(&p, &r, SourceImageLimits::default(), &|| false).is_err());
        assert_eq!(reads.get(), 0, "mutation {mutation}");
    }
    reads.set(0);
    assert_eq!(
        extract(&p, &good, SourceImageLimits::default(), &|| false).unwrap(),
        support::resources().0
    );
    assert!(reads.get() > 0);
    assert_eq!(
        serde_json::to_value(&good.resources[0]).unwrap()["offset"],
        json!("0")
    );
}
