use super::*;
use crate::geometry::test_support::{Backend, FONT};
use mo_common::{ByteLength, Digest};
use mo_unicode::bidi::ParagraphDirection;
#[path = "prepared_tests.rs"]
mod prepared_tests;

fn request(text: &str) -> ManifestParagraphRequest {
    let digest = Digest::try_from(
        "5e182a1675e0255bce1ff90e6a00d1fee0c12acee868987af1b4f39bf27cabc9".to_owned(),
    )
    .unwrap();
    let font =
        mo_font::VerifiedFont::load(&digest, 0, FONT, mo_font::FontLimits::default(), &|| false)
            .unwrap();
    let name = |id, expected: &str| FontNameBinding {
        record: font
            .metadata()
            .names
            .iter()
            .position(|n| n.name_id == id)
            .unwrap() as u32,
        expected: expected.into(),
    };
    let family = "MusterOffice Synthetic";
    ManifestParagraphRequest {
        manifest: FontManifest {
            profile: FontManifestProfile::ExplicitDraftV1,
            fonts: vec![CascadeFont {
                expected_sha256: digest,
                face_index: 0,
                offset: ByteLength::new(0),
                byte_length: ByteLength::new(FONT.len() as u64),
            }],
            faces: vec![ManifestFace {
                font: 0,
                family: name(1, family),
                subfamily: name(2, "Regular"),
                postscript: Some(name(6, "MusterOfficeSynthetic")),
            }],
            typefaces: vec![ManifestTypeface {
                typeface: family.into(),
                policy: TypefaceMappingPolicy::ExactFamily {},
                regular: Some(ManifestInstance {
                    face: 0,
                    variations: vec![],
                }),
                bold: None,
                italic: None,
                bold_italic: None,
            }],
        },
        text: text.into(),
        direction: ParagraphDirection::LeftToRight,
        spans: if text.is_empty() {
            vec![]
        } else {
            vec![itemize::StyleSpan {
                end: text.chars().count() as u32,
                style: 0,
            }]
        },
        styles: vec![ManifestTextStyle {
            typeface: family.into(),
            font_style: FontStyle::Regular,
            language: "en".into(),
            features: vec![],
            suppress_dotted_circle: false,
            max_glyphs: 64,
        }],
    }
}
fn reject(q: &ManifestParagraphRequest) {
    let mut backend = Backend::default();
    assert!(shape_paragraph(q, FONT, &mut backend, ManifestLimits::default(), &|| false).is_err());
    assert_eq!(backend.shapes, 0);
    assert!(!backend.invalid);
}
#[test]
fn names_slots_and_axes_are_bound_before_paragraph_shaping() {
    let mut q = request("AAA");
    q.manifest.typefaces[0].regular.as_mut().unwrap().variations = vec![ShapeVariation {
        tag: "wght".into(),
        value_16_16: 450 * 65536,
    }];
    let mut b = Backend::default();
    let r = shape_paragraph(&q, FONT, &mut b, ManifestLimits::default(), &|| false).unwrap();
    assert_eq!(r.bindings.len(), 1);
    assert_eq!(r.bindings[0].candidate.font, 0);
    assert_eq!(
        r.bindings[0].candidate.variations[0].value_16_16,
        450 * 65536
    );
    assert_eq!(r.shaping.fallback.verified_faces, 1);
    assert!(b.shapes > 0);
}
#[test]
fn unused_face_names_and_unused_slot_axes_cannot_hide_behind_coverage() {
    for text in ["A", "\t", ""] {
        let mut q = request(text);
        let mut extra = q.manifest.faces[0].clone();
        extra.family.expected = "Wrong family".into();
        q.manifest.faces.push(extra);
        reject(&q);
        let mut q = request(text);
        q.manifest.typefaces[0].bold = Some(ManifestInstance {
            face: 0,
            variations: vec![ShapeVariation {
                tag: "xxxx".into(),
                value_16_16: 0,
            }],
        });
        reject(&q);
        let mut q = request(text);
        q.manifest.typefaces[0].regular.as_mut().unwrap().variations = vec![ShapeVariation {
            tag: "wght".into(),
            value_16_16: 901 * 65536,
        }];
        reject(&q);
        let mut q = request(text);
        q.manifest.faces[0].family.record = q.manifest.faces[0].subfamily.record;
        q.manifest.faces[0].family.expected = "Regular".into();
        reject(&q);
    }
}
#[test]
fn no_implicit_family_or_style_substitution_is_performed() {
    let mut q = request("A");
    q.styles[0].font_style = FontStyle::Bold;
    reject(&q);
    let mut q = request("A");
    q.styles[0].typeface = "Missing".into();
    reject(&q);
    let mut q = request("A");
    q.styles[0].typeface = "Source family".into();
    q.manifest.typefaces[0].typeface = "Source family".into();
    reject(&q);
    q.manifest.typefaces[0].policy = TypefaceMappingPolicy::Substitution {
        profile_sha256: Digest::from_sha256([7; 32]),
        reason: "Owned explicit substitution probe".into(),
    };
    let r = shape_paragraph(
        &q,
        FONT,
        &mut Backend::default(),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        r.bindings[0].policy,
        TypefaceMappingPolicy::Substitution { .. }
    ));
    q.manifest.typefaces.push(q.manifest.typefaces[0].clone());
    reject(&q);
}
#[test]
fn resource_conflicts_limits_and_cancellation_fail_before_component() {
    let q = request("A");
    let mut b = Backend::default();
    let mut wrong = q.clone();
    wrong.manifest.fonts[0].expected_sha256 = Digest::from_sha256([0; 32]);
    reject(&wrong);
    for limits in [
        ManifestLimits {
            max_fonts: 0,
            ..Default::default()
        },
        ManifestLimits {
            max_faces: 0,
            ..Default::default()
        },
        ManifestLimits {
            max_typefaces: 0,
            ..Default::default()
        },
        ManifestLimits {
            max_name_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(shape_paragraph(&q, FONT, &mut b, limits, &|| false).is_err());
    }
    assert!(
        shape_paragraph(&q, &FONT[..10], &mut b, ManifestLimits::default(), &|| {
            false
        })
        .is_err()
    );
    assert!(matches!(
        shape_paragraph(&q, FONT, &mut b, ManifestLimits::default(), &|| true),
        Err(TextError::Cancelled)
    ));
    assert_eq!(b.shapes, 0);
}
#[test]
fn control_only_input_validates_resources_without_shaping() {
    let mut b = Backend::default();
    let r = shape_paragraph(
        &request("\t\r\n"),
        FONT,
        &mut b,
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(b.shapes, 0);
    assert!(r.shaping.shaped_item_indices.is_empty());
    assert_eq!(r.shaping.fallback.verified_faces, 1);
}

#[test]
fn missing_selection_names_the_required_style_without_invalidating_the_backend() {
    for missing_name in [false, true] {
        let mut q = request("A");
        q.styles[0].font_style = FontStyle::BoldItalic;
        if missing_name {
            q.styles[0].typeface = "Required source family".into();
        }
        let mut backend = Backend::default();
        let Err(TextError::FontSelection(failure)) =
            shape_paragraph(&q, FONT, &mut backend, ManifestLimits::default(), &|| false)
        else {
            panic!("expected a typed font requirement")
        };
        assert_eq!(failure.style, 0);
        assert_eq!(failure.typeface, q.styles[0].typeface);
        assert_eq!(failure.font_style, FontStyle::BoldItalic);
        assert_eq!(
            failure.reason,
            if missing_name {
                FontSelectionReason::UnmappedTypeface
            } else {
                FontSelectionReason::MissingStyle
            }
        );
        assert_eq!(backend.shapes, 0);
        assert!(!backend.invalid);
    }
}

#[test]
fn malformed_and_oversized_names_do_not_become_font_requests() {
    let mut q = request("A");
    q.styles[0].typeface.clear();
    let mut backend = Backend::default();
    assert!(matches!(
        shape_paragraph(&q, FONT, &mut backend, ManifestLimits::default(), &|| false),
        Err(TextError::Invalid(_))
    ));
    q.styles[0].typeface = "x".repeat(ManifestLimits::default().max_name_bytes + 1);
    assert!(matches!(
        shape_paragraph(&q, FONT, &mut backend, ManifestLimits::default(), &|| false),
        Err(TextError::Limit(_))
    ));
    assert_eq!(backend.shapes, 0);
}
