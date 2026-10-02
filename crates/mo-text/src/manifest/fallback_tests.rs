use super::*;

fn fallback_request() -> ManifestParagraphRequest {
    let mut q = request("A");
    // A second explicit style/axis instance of the owned synthetic font.
    q.manifest.fonts.push(q.manifest.fonts[0].clone());
    let mut face = q.manifest.faces[0].clone();
    face.font = 1;
    q.manifest.faces.push(face);
    let mut fallback = q.manifest.typefaces[0].clone();
    fallback.typeface = "Owned fallback".into();
    fallback.policy = TypefaceMappingPolicy::Substitution {
        profile_sha256: Digest::from_sha256([3; 32]),
        reason: "Owned fallback coverage test".into(),
    };
    fallback.regular.as_mut().unwrap().face = 1;
    fallback.regular.as_mut().unwrap().variations = vec![ShapeVariation {
        tag: "wght".into(),
        value_16_16: 700 * 65536,
    }];
    q.manifest.typefaces[0]
        .fallbacks
        .push(fallback.typeface.clone());
    q.manifest.typefaces.push(fallback);
    q
}

#[derive(Default)]
struct MissingPrimary(Backend);
impl backend::TextBackend for MissingPrimary {
    fn shape_batch(&mut self, bundle: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        let first = self.0.shapes == 0;
        let mut reply = self.0.shape_batch(bundle, frame)?;
        if first {
            // One scalar, one run: missing glyph forces the ordered fallback.
            assert_eq!(reply.len(), 18);
            reply[11] = 0;
        }
        Ok(reply)
    }
    fn invalidate(&mut self) {
        self.0.invalid = true;
    }
}

#[test]
fn explicit_fallback_reaches_shaping_and_retains_candidate_evidence() {
    let q = fallback_request();
    let mut backend = MissingPrimary::default();
    let r = shape_paragraph(&q, FONT, &mut backend, ManifestLimits::default(), &|| false).unwrap();
    assert_eq!(r.bindings.len(), 1);
    assert_eq!(r.bindings[0].candidate.font, 0);
    let fallback = &r.bindings[0].fallbacks[0];
    assert_eq!(
        (fallback.typeface, fallback.face, fallback.candidate.font),
        (1, 1, 1)
    );
    assert_eq!(fallback.candidate.variations[0].value_16_16, 700 * 65536);
    assert!(matches!(
        fallback.policy,
        TypefaceMappingPolicy::Substitution { .. }
    ));
    let crate::fallback::FontFragment::Selected {
        font, candidate, ..
    } = &r.shaping.fallback.items[0].fragments[0]
    else {
        panic!("explicit fallback must supply the missing glyph");
    };
    assert_eq!((*font, *candidate), (1, 1));
    assert_eq!(
        r.shaping.fallback.items[0].probes[0].missing_glyph_clusters,
        vec![0]
    );
    assert_eq!(backend.0.shapes, 2);
    assert!(!backend.0.invalid);
}

#[test]
fn covered_primary_is_retained_and_fallbacks_are_not_recursive() {
    let mut q = fallback_request();
    // Flat candidate lists can be symmetric without recursive expansion.
    q.manifest.typefaces[1].fallbacks = vec![q.manifest.typefaces[0].typeface.clone()];
    let mut backend = Backend::default();
    let r = shape_paragraph(&q, FONT, &mut backend, ManifestLimits::default(), &|| false).unwrap();
    assert_eq!(r.bindings[0].fallbacks.len(), 1);
    assert_eq!(backend.shapes, 1);
    assert!(matches!(
        r.shaping.fallback.items[0].fragments[0],
        crate::fallback::FontFragment::Selected {
            font: 0,
            candidate: 0,
            ..
        }
    ));
}

#[test]
fn bad_or_excessive_fallback_references_are_rejected_before_shaping() {
    for names in [
        vec!["Absent".into()],
        vec!["MusterOffice Synthetic".into()],
        vec!["Owned fallback".into(); 2],
        vec!["Owned fallback".into(); 32],
    ] {
        let mut q = fallback_request();
        q.manifest.typefaces[0].fallbacks = names;
        reject(&q);
    }
    let mut q = fallback_request();
    q.manifest.typefaces[1].regular.as_mut().unwrap().variations[0].value_16_16 = 901 * 65536;
    reject(&q); // Even if the primary font covers the complete input.
}

#[test]
fn fallback_style_is_explicit_and_never_synthesized() {
    let mut q = fallback_request();
    q.manifest.typefaces[0].bold = q.manifest.typefaces[0].regular.clone();
    q.styles[0].font_style = FontStyle::Bold;
    let mut backend = Backend::default();
    let Err(TextError::FontSelection(error)) =
        shape_paragraph(&q, FONT, &mut backend, ManifestLimits::default(), &|| false)
    else {
        panic!("missing fallback style must be diagnosed");
    };
    assert_eq!(error.typeface, "Owned fallback");
    assert_eq!(error.font_style, FontStyle::Bold);
    assert_eq!(error.reason, FontSelectionReason::MissingStyle);
    assert_eq!(backend.shapes, 0);
}

#[test]
fn existing_manifests_preserve_their_wire_representation_and_selection() {
    let q = request("A");
    let value = serde_json::to_value(&q).unwrap();
    assert!(value["manifest"]["typefaces"][0].get("fallbacks").is_none());
    let old: ManifestParagraphRequest = serde_json::from_value(value.clone()).unwrap();
    assert!(old.manifest.typefaces[0].fallbacks.is_empty());
    assert_eq!(serde_json::to_value(&old).unwrap(), value);
    let result = shape_paragraph(
        &old,
        FONT,
        &mut Backend::default(),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(
        serde_json::to_value(&result).unwrap()["bindings"][0]
            .get("fallbacks")
            .is_none()
    );
}
