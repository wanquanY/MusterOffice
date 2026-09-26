//! Name, instance and per-paragraph style contracts.
use super::*;
fn invalid() -> TextError {
    TextError::Invalid("font manifest binding")
}
pub(super) fn structure<'a>(
    manifest: &'a FontManifest,
    limits: ManifestLimits,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<&'a str, usize>, TextError> {
    cancelled(check)?;
    if manifest.fonts.len() > limits.max_fonts
        || manifest.faces.len() > limits.max_faces
        || manifest.typefaces.len() > limits.max_typefaces
    {
        return Err(TextError::Limit("font manifest entries"));
    }
    let mut bytes = 0usize;
    let mut name = |s: &str| -> Result<(), TextError> {
        if s.is_empty() {
            return Err(invalid());
        }
        bytes = bytes
            .checked_add(s.len())
            .ok_or(TextError::Limit("font manifest names"))?;
        if bytes > limits.max_name_bytes {
            return Err(TextError::Limit("font manifest names"));
        }
        Ok(())
    };
    for face in &manifest.faces {
        cancelled(check)?;
        if face.font as usize >= manifest.fonts.len() {
            return Err(invalid());
        }
        for n in [
            Some(&face.family),
            Some(&face.subfamily),
            face.postscript.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            name(&n.expected)?;
        }
    }
    let mut names = BTreeMap::new();
    for (i, typeface) in manifest.typefaces.iter().enumerate() {
        cancelled(check)?;
        name(&typeface.typeface)?;
        if names.insert(typeface.typeface.as_str(), i).is_some() {
            return Err(TextError::Invalid("duplicate manifest typeface"));
        }
        if let TypefaceMappingPolicy::Substitution { reason, .. } = &typeface.policy {
            name(reason)?;
        }
        for instance in typeface.slots().into_iter().flatten() {
            cancelled(check)?;
            if instance.face as usize >= manifest.faces.len() {
                return Err(invalid());
            }
            if instance.variations.len() > 64 {
                return Err(TextError::Limit("font manifest axes"));
            }
            for axis in &instance.variations {
                name(&axis.tag)?;
            }
        }
    }
    Ok(names)
}
fn verify_name(
    binding: &FontNameBinding,
    ids: &[u16],
    font: &mo_font::FontInspection,
) -> Result<(), TextError> {
    let actual = font
        .names
        .get(binding.record as usize)
        .ok_or_else(invalid)?;
    if !ids.contains(&actual.name_id) || actual.text.as_ref() != Some(&binding.expected) {
        return Err(TextError::Invalid(
            "font manifest name differs from verified font",
        ));
    }
    Ok(())
}
pub(super) fn verify(
    manifest: &FontManifest,
    fonts: &[VerifiedFont<'_>],
    bindings: &[usize],
    check: &dyn Fn() -> bool,
) -> Result<(), TextError> {
    for face in &manifest.faces {
        cancelled(check)?;
        let font = fonts
            .get(*bindings.get(face.font as usize).ok_or_else(invalid)?)
            .ok_or_else(invalid)?
            .metadata();
        verify_name(&face.family, &[1, 16, 21], font)?;
        verify_name(&face.subfamily, &[2, 17, 22], font)?;
        if let Some(name) = &face.postscript {
            verify_name(name, &[6], font)?;
        }
    }
    let base = CascadeItem {
        start: 0,
        end: 0,
        direction: Direction::LeftToRight,
        script: "Zyyy".into(),
        language: "und".into(),
        features: vec![],
        beginning_of_text: true,
        end_of_text: true,
        suppress_dotted_circle: false,
        max_glyphs: 1,
        candidates: vec![],
    };
    for typeface in &manifest.typefaces {
        for instance in typeface.slots().into_iter().flatten() {
            cancelled(check)?;
            let face = &manifest.faces[instance.face as usize];
            if matches!(typeface.policy, TypefaceMappingPolicy::ExactFamily {})
                && typeface.typeface != face.family.expected
            {
                return Err(TextError::Invalid(
                    "typeface requires an explicit substitution profile",
                ));
            }
            let font = fonts[bindings[face.font as usize]].metadata();
            // Same axis syntax/range/duplicate checks used by every shaping path.
            let candidate = FontCandidate {
                font: face.font,
                variations: instance.variations.clone(),
            };
            prepare::validate_run(&base.run(&candidate), font, 0, TextLimits::default())?;
        }
    }
    Ok(())
}

pub(super) fn paragraph(
    manifest: &FontManifest,
    names: &BTreeMap<&str, usize>,
    request: ManifestParagraphInput<'_>,
    limits: ManifestLimits,
    check: &dyn Fn() -> bool,
) -> Result<(ParagraphShapeRequest, Vec<ManifestStyleBinding>), TextError> {
    // Apply paragraph limits before cloning user-owned styles/text into the
    // existing wire representation. Keep byte/scalar meanings distinct.
    if request.text.len() > 262144 || request.styles.len() > 256 || request.spans.len() > 256 {
        return Err(TextError::Limit("manifest paragraph size"));
    }
    let mut styles = vec![];
    let mut selected = vec![];
    let mut copied_policy_bytes = 0usize;
    for (i, s) in request.styles.iter().enumerate() {
        cancelled(check)?;
        if s.typeface.len() > limits.max_name_bytes
            || s.language.len() > 255
            || s.features.len() > 1024
            || s.features.iter().any(|f| f.tag.len() > 4)
        {
            return Err(TextError::Limit("manifest paragraph style"));
        }
        if s.typeface.is_empty() {
            return Err(TextError::Invalid("empty manifest typeface"));
        }
        let missing = |reason| {
            TextError::FontSelection(Box::new(FontSelectionFailure {
                style: i as u32,
                typeface: s.typeface.clone(),
                font_style: s.font_style,
                reason,
            }))
        };
        let &typeface = names
            .get(s.typeface.as_str())
            .ok_or_else(|| missing(FontSelectionReason::UnmappedTypeface))?;
        let mapping = &manifest.typefaces[typeface];
        if let TypefaceMappingPolicy::Substitution { reason, .. } = &mapping.policy {
            copied_policy_bytes = copied_policy_bytes
                .checked_add(reason.len())
                .ok_or(TextError::Limit("manifest result policy bytes"))?;
            if copied_policy_bytes > limits.max_name_bytes {
                return Err(TextError::Limit("manifest result policy bytes"));
            }
        }
        let instance = mapping
            .slot(s.font_style)
            .ok_or_else(|| missing(FontSelectionReason::MissingStyle))?;
        let candidate = FontCandidate {
            font: manifest.faces[instance.face as usize].font,
            variations: instance.variations.clone(),
        };
        styles.push(ParagraphTextStyle {
            language: s.language.clone(),
            features: s.features.clone(),
            candidates: vec![candidate.clone()],
            suppress_dotted_circle: s.suppress_dotted_circle,
            max_glyphs: s.max_glyphs,
        });
        selected.push(ManifestStyleBinding {
            style: i as u32,
            typeface: typeface as u32,
            font_style: s.font_style,
            face: instance.face,
            candidate,
            policy: mapping.policy.clone(),
        });
    }
    let paragraph = ParagraphShapeRequest {
        text: request.text.to_owned(),
        direction: request.direction,
        spans: request.spans.to_vec(),
        styles,
        fonts: manifest.fonts.clone(),
    };
    Ok((paragraph, selected))
}
