use crate::*;
use std::collections::BTreeSet;

pub(crate) fn validate(d: &Document, max_issues: usize) -> ValidationReport {
    let Some(b) = &d.source_bindings else {
        return ValidationReport::default();
    };
    let mut report = ValidationReport::default();
    let mut require = |ok: bool, message: &str| {
        if !ok {
            if report.issues.len() >= max_issues {
                report.truncated = true;
                return;
            }
            report.issues.push(ValidationIssue {
                code: ValidationCode::InvalidValue,
                path: "/sourceBindings".into(),
                message: message.into(),
            });
        }
    };
    require(
        d.resources
            .get(&b.resource)
            .is_some_and(|r| r.kind == ResourceKind::SourcePackage),
        "source binding requires source-package resource",
    );
    require(
        b.slides.keys().eq(d.slides.keys())
            && b.masters.keys().eq(d.masters.keys())
            && b.layouts.keys().eq(d.layouts.keys())
            && b.themes.keys().eq(d.themes.keys()),
        "source surface bindings must cover document surfaces exactly",
    );
    require(
        b.objects.keys().eq(d.objects.keys()),
        "source object bindings must cover objects exactly",
    );
    let mut addresses = BTreeSet::new();
    for (id, binding) in &b.objects {
        require(
            addresses.insert((&binding.part, binding.native_id)),
            "duplicate native object address",
        );
        let Some(o) = d.objects.get(id) else { continue };
        let ObjectContent::RetainedSource { paragraphs, .. } = &o.content else {
            require(false, "source-bound object requires retained semantics");
            continue;
        };
        require(
            o.transform.is_some() || binding.transform_constraint.is_some(),
            "missing direct transform must be protected",
        );
        if let Some(t) = o.transform {
            require(
                [t.origin.x, t.origin.y].iter().all(|v| {
                    (PRESENTATIONML_COORDINATE_MIN..=PRESENTATIONML_COORDINATE_MAX)
                        .contains(&v.get())
                }) && [t.size.width, t.size.height]
                    .iter()
                    .all(|v| (0..=PRESENTATIONML_COORDINATE_MAX).contains(&v.get())),
                "retained transform exceeds native coordinate range",
            );
        }
        let mut runs = BTreeSet::new();
        for (pi, p) in paragraphs.iter().enumerate() {
            for (ri, r) in p.runs.iter().enumerate() {
                runs.insert(&r.id);
                require(r.text.chars().all(|c| matches!(c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')),
                    "retained text contains an invalid native XML character");
                require(
                    binding.runs.get(&r.id).is_some_and(|n| {
                        n.paragraph as usize == pi
                            && n.run as usize == ri
                            && (r.kind == RetainedRunKind::Text || n.constraint.is_some())
                    }),
                    "retained run binding, position or edit constraint is invalid",
                );
            }
        }
        require(
            runs.into_iter().eq(binding.runs.keys()),
            "retained run bindings must cover runs exactly",
        );
    }
    report
}
