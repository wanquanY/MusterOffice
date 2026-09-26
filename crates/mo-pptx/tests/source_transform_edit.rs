mod support;
use mo_common::Emu;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{source::*, *};
use mo_presentation_model::{Point, Size};
use std::cell::Cell;

const SLIDE: &str = "/ppt/slides/slide1.xml";
fn fixture() -> Vec<u8> {
    let (doc, defaults) = support::input();
    export(
        &doc,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn xml(bytes: &[u8], part: &str) -> String {
    String::from_utf8(
        package(bytes)
            .read_part(&PartName::new(part).unwrap(), 1024 * 1024, &|| false)
            .unwrap(),
    )
    .unwrap()
}
fn change(bytes: &[u8], part: &str, mutate: impl FnOnce(String) -> String) -> Vec<u8> {
    let mut plan = RewritePlan::new();
    plan.replace_part(
        PartName::new(part).unwrap(),
        mutate(xml(bytes, part)).into_bytes(),
    )
    .unwrap();
    plan.to_bytes(&package(bytes), &|| false).unwrap()
}
fn index(bytes: &[u8]) -> SourceIndex {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false).unwrap()
}
fn request(bytes: &[u8], name: &str) -> SourceTransformEdits {
    let idx = index(bytes);
    let (part, object) = idx
        .surfaces
        .iter()
        .find_map(|(p, s)| s.objects.iter().find(|o| o.name == name).map(|o| (p, o)))
        .unwrap();
    let expected = SourceTransformValues::from(object.transform.as_ref().unwrap());
    SourceTransformEdits {
        expected_source_sha256: idx.source_sha256,
        edits: vec![SourceTransformEdit {
            target: SourceObjectRef {
                part: part.clone(),
                native_id: object.native_id,
            },
            replacement: expected.clone(),
            expected,
        }],
    }
}
fn edit(bytes: &[u8], request: &SourceTransformEdits) -> Result<Vec<u8>, PptxError> {
    edit_source_transforms(&package(bytes), request, SourceLimits::default(), &|| false)
}

#[test]
fn transform_changes_preserve_unknown_parts_text_and_unselected_attribute_tokens() {
    let bytes = change(&fixture(), SLIDE, |s| {
        s.replace(
            "<p:cSld",
            "<!-- exact --><p:cSld xmlns:q=\"urn:owned\" q:keep=\"a&amp;b\"",
        )
    });
    let mut r = request(&bytes, "title:1");
    let target = &mut r.edits[0].replacement;
    target.rotation = Some(-8_100_000);
    target.flip_horizontal = Some(true);
    target.flip_vertical = Some(false);
    target.origin = Some(Point {
        x: Emu::new(-700),
        y: Emu::new(42),
    });
    target.size = Some(Size {
        width: Emu::new(2000),
        height: Emu::new(0),
    });
    let out = edit(&bytes, &r).unwrap();
    let idx = index(&out);
    let o = idx.surfaces[SLIDE]
        .objects
        .iter()
        .find(|o| o.native_id == r.edits[0].target.native_id)
        .unwrap();
    assert_eq!(
        SourceTransformValues::from(o.transform.as_ref().unwrap()),
        r.edits[0].replacement
    );
    assert!(
        xml(&out, SLIDE).contains("<!-- exact --><p:cSld xmlns:q=\"urn:owned\" q:keep=\"a&amp;b\"")
    );
    let before = package(&bytes);
    let after = package(&out);
    assert_eq!(before.relationships(), after.relationships());
    for (part, info) in before.parts() {
        if part.as_str() != SLIDE {
            assert_eq!(info.sha256, after.parts()[part].sha256);
        }
    }
    let original = index(&bytes);
    assert_eq!(
        o.paragraphs,
        original.surfaces[SLIDE]
            .objects
            .iter()
            .find(|v| v.native_id == o.native_id)
            .unwrap()
            .paragraphs
    );
    assert!(matches!(edit(&out, &r), Err(PptxError::SourceConflict(_))));
}

#[test]
fn noops_and_semantically_equal_native_tokens_keep_exact_original_package() {
    let bytes = change(&fixture(), "/ppt/slides/slide2.xml", |s| {
        s.replace(
            "rot=\"21300000\" flipH=\"0\"",
            "rot = '+021300000' flipH='false'",
        )
    });
    let r = request(&bytes, "group:2");
    assert_eq!(edit(&bytes, &r).unwrap(), bytes);
    let mut changed = r.clone();
    changed.edits[0].replacement.rotation = Some(i32::MIN);
    let out = edit(&bytes, &changed).unwrap();
    assert_eq!(
        xml(&out, &changed.edits[0].target.part),
        xml(&bytes, &changed.edits[0].target.part).replace("'+021300000'", "'-2147483648'")
    );
    let mut clear = request(&out, "group:2");
    clear.edits[0].replacement.rotation = None;
    clear.edits[0].replacement.flip_horizontal = None;
    let cleared = edit(&out, &clear).unwrap();
    assert!(!xml(&cleared, &clear.edits[0].target.part).contains("rot ="));
}

#[test]
fn group_child_coordinates_root_transform_and_multiple_parts_share_one_transaction() {
    let bytes = change(&fixture(), SLIDE, |s| {
        s.replacen("<p:grpSpPr/>", "<p:grpSpPr><a:xfrm/></p:grpSpPr>", 1)
    });
    let idx = index(&bytes);
    let mut r = request(&bytes, "group:2");
    r.edits[0].replacement.child_origin = Some(Point {
        x: Emu::new(-7),
        y: Emu::new(8),
    });
    r.edits[0].replacement.child_size = Some(Size {
        width: Emu::new(123),
        height: Emu::new(456),
    });
    r.edits[0].replacement.rotation = Some(i32::MAX);
    let surface = &idx.surfaces[SLIDE];
    let expected = SourceTransformValues::from(surface.root_group_transform.as_ref().unwrap());
    let mut replacement = expected.clone();
    replacement.flip_vertical = Some(true);
    r.edits.push(SourceTransformEdit {
        target: SourceObjectRef {
            part: SLIDE.into(),
            native_id: surface.root_object_id,
        },
        expected,
        replacement,
    });
    let out = edit(&bytes, &r).unwrap();
    let after = index(&out);
    assert_eq!(
        after.surfaces[SLIDE]
            .root_group_transform
            .as_ref()
            .unwrap()
            .flip_vertical,
        Some(true)
    );
    assert_eq!(
        request(&out, "group:2").edits[0].expected,
        r.edits[0].replacement
    );
    r.edits.reverse();
    assert_eq!(edit(&bytes, &r).unwrap(), out);
}

#[test]
fn conflicting_preconditions_missing_targets_and_duplicate_objects_are_atomic() {
    let bytes = fixture();
    let mut r = request(&bytes, "title:1");
    r.edits[0].replacement.rotation = Some(1);
    for kind in 0..5 {
        let mut bad = r.clone();
        match kind {
            0 => bad.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]),
            1 => bad.edits[0].expected.rotation = Some(1),
            2 => bad.edits[0].target.native_id = u32::MAX,
            3 => bad.edits[0].target.part = "/ppt/slides/absent.xml".into(),
            _ => bad.edits.push(bad.edits[0].clone()),
        }
        assert!(
            matches!(edit(&bytes, &bad), Err(PptxError::SourceConflict(_))),
            "{kind}"
        );
    }
    let mut second = request(&bytes, "title:2").edits.remove(0);
    second.expected.rotation = Some(700);
    r.edits.push(second);
    assert!(edit(&bytes, &r).is_err());
    assert_eq!(fixture(), bytes);
}

#[test]
fn structural_leaf_changes_and_invalid_native_ranges_are_not_silently_clamped() {
    let bytes = fixture();
    let r = request(&bytes, "title:1");
    for value in [i64::MIN, -27_273_042_329_601, 27_273_042_316_901, i64::MAX] {
        let mut bad = r.clone();
        bad.edits[0].replacement.origin.as_mut().unwrap().x = Emu::new(value);
        assert!(matches!(
            edit(&bytes, &bad),
            Err(PptxError::SourceConflict(_))
        ));
    }
    for value in [-1, 27_273_042_316_901] {
        let mut bad = r.clone();
        bad.edits[0].replacement.size.as_mut().unwrap().height = Emu::new(value);
        assert!(matches!(
            edit(&bytes, &bad),
            Err(PptxError::SourceConflict(_))
        ));
    }
    for coordinate in [-27_273_042_329_600, 27_273_042_316_900] {
        let mut good = r.clone();
        good.edits[0].replacement.origin.as_mut().unwrap().x = Emu::new(coordinate);
        assert!(edit(&bytes, &good).is_ok());
    }
    let mut bad = r.clone();
    bad.edits[0].replacement.origin = None;
    assert!(matches!(edit(&bytes, &bad), Err(PptxError::Unsupported(_))));
    let mut bad = r.clone();
    bad.edits[0].replacement.child_origin = Some(Point {
        x: Emu::ZERO,
        y: Emu::ZERO,
    });
    assert!(matches!(edit(&bytes, &bad), Err(PptxError::Unsupported(_))));
}

#[test]
fn ignored_transform_material_and_alternate_representations_require_coordination() {
    for token in [
        "<a:xfrm xmlns:q=\"urn:future\" q:cached=\"yes\">",
        "<a:xfrm xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:q=\"urn:future\" mc:Ignorable=\"q\" q:cached=\"yes\">",
        "<a:xfrm xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:q=\"urn:future\" mc:Ignorable=\"q\"><q:cached/>",
    ] {
        let bytes = change(&fixture(), SLIDE, |s| {
            s.replacen("<a:xfrm rot=\"0\" flipH=\"0\" flipV=\"0\">", token, 1)
        });
        let mut r = request(&bytes, "title:1");
        assert_eq!(edit(&bytes, &r).unwrap(), bytes);
        r.edits[0].replacement.rotation = Some(10);
        assert!(edit(&bytes, &r).is_err());
    }
    let bytes = change(&fixture(), SLIDE, |s| {
        let start = s.find("<p:sp>").unwrap();
        let end = start + s[start..].find("</p:sp>").unwrap() + 7;
        let shape = &s[start..end];
        let wrapper = format!(
            "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:q=\"urn:future\"><mc:Choice Requires=\"q\">{shape}</mc:Choice><mc:Fallback>{shape}</mc:Fallback></mc:AlternateContent>"
        );
        format!("{}{}{}", &s[..start], wrapper, &s[end..])
    });
    let mut r = request(&bytes, "title:1");
    r.edits[0].replacement.rotation = Some(10);
    assert!(matches!(edit(&bytes, &r), Err(PptxError::Unsupported(_))));
    let mut r = request(&bytes, "unicode:1");
    r.edits[0].replacement.rotation = Some(10);
    assert!(edit(&bytes, &r).is_ok());
}

#[test]
fn changes_to_layout_declarations_recompute_inherited_geometry_without_materializing_it() {
    let layout = "/ppt/slideLayouts/slideLayout2.xml";
    let bytes = change(&fixture(), layout, |s| {
        s.replacen(
            "<p:cNvSpPr/><p:nvPr/>",
            "<p:cNvSpPr/><p:nvPr><p:ph idx=\"7\"/></p:nvPr>",
            1,
        )
    });
    let bytes = change(&bytes, SLIDE, |s| {
        let start = s.find("<a:xfrm ").unwrap();
        let end = start + s[start..].find("</a:xfrm>").unwrap() + 9;
        let s = format!("{}{}", &s[..start], &s[end..]);
        s.replacen(
            "<p:cNvSpPr/><p:nvPr/>",
            "<p:cNvSpPr/><p:nvPr><p:ph idx=\"7\"/></p:nvPr>",
            1,
        )
    });
    let before = index(&bytes);
    let child = &before.surfaces[SLIDE].objects[0];
    assert!(child.transform.is_none());
    assert!(child.resolution.origin.is_some());
    let mut r = request(&bytes, "rule:layout");
    r.edits[0].replacement.origin.as_mut().unwrap().x = Emu::new(9898);
    let out = edit(&bytes, &r).unwrap();
    let after = index(&out);
    let child = &after.surfaces[SLIDE].objects[0];
    assert!(child.transform.is_none());
    assert_eq!(
        child.resolution.origin.as_ref().unwrap().value.x.get(),
        9898
    );
    assert_eq!(xml(&out, SLIDE), xml(&bytes, SLIDE));
}

#[test]
fn editing_respects_limits_and_cancellation_before_candidate_publication() {
    let bytes = fixture();
    let source = package(&bytes);
    let mut r = request(&bytes, "title:1");
    r.edits[0].replacement.rotation = Some(77);
    for limits in [
        SourceLimits {
            max_edits: 0,
            ..Default::default()
        },
        SourceLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            edit_source_transforms(&source, &r, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    let calls = Cell::new(0);
    edit_source_transforms(&source, &r, SourceLimits::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in [1, 2, 10, calls.get() / 2, calls.get() - 1, calls.get()] {
        let n = Cell::new(0);
        let result = edit_source_transforms(&source, &r, SourceLimits::default(), &|| {
            n.set(n.get() + 1);
            n.get() == stop
        });
        assert!(result.is_err(), "cancel {stop}/{}", calls.get());
    }
}
