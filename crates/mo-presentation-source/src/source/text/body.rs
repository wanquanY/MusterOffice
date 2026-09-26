//! Per-property text body inheritance. Does not perform text layout or fitting.
mod types;
use super::*;
use crate::{PptxError, cancelled, source::*, value};
use theme::SourceThemeDefaultKind;
pub use types::*;

struct Budget<'a> {
    limits: TextBodyLimits,
    check: &'a dyn Fn() -> bool,
    steps: usize,
    bytes: usize,
}
impl Budget<'_> {
    fn step(&mut self) -> Result<(), PptxError> {
        cancelled(self.check)?;
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(PptxError::Limit("text body steps"))?;
        if self.steps > self.limits.max_steps {
            return Err(PptxError::Limit("text body steps"));
        }
        Ok(())
    }
    fn bytes(&mut self, n: usize) -> Result<(), PptxError> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .ok_or(PptxError::Limit("text body bytes"))?;
        if self.bytes > self.limits.max_lexical_bytes {
            return Err(PptxError::Limit("text body bytes"));
        }
        Ok(())
    }
}
enum Failure {
    Unresolved(TextBodyUnresolved),
    Abort(PptxError),
}
impl From<PptxError> for Failure {
    fn from(e: PptxError) -> Self {
        Self::Abort(e)
    }
}
impl From<TextBodyUnresolved> for Failure {
    fn from(e: TextBodyUnresolved) -> Self {
        Self::Unresolved(e)
    }
}
fn conflict() -> PptxError {
    PptxError::SourceConflict("text body binding differs from inspected source".into())
}
fn node(catalog: &SourceTextCatalog, id: u32) -> Result<&SourceTextNode, PptxError> {
    catalog.nodes.get(&id).ok_or_else(conflict)
}
fn child(
    catalog: &SourceTextCatalog,
    id: u32,
    kind: NativeTextElement,
    budget: &mut Budget<'_>,
) -> Result<Option<u32>, PptxError> {
    for id in &node(catalog, id)?.children {
        budget.step()?;
        if node(catalog, *id)?.element == kind {
            return Ok(Some(*id));
        }
    }
    Ok(None)
}
#[derive(Default)]
struct Partial {
    values: SourceTextBodyAttributes,
    origins: BTreeMap<TextBodyProperty, TextBodyOrigin>,
    autofit: Option<EffectiveTextAutofit>,
}
impl Partial {
    fn complete(&self) -> bool {
        self.origins.len() == 19 && self.autofit.is_some()
    }
    fn merge(
        &mut self,
        catalog: &SourceTextCatalog,
        id: u32,
        origin: TextBodyOrigin,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        budget.step()?;
        let n = node(catalog, id)?;
        let SourceTextValue::Body { attributes } = &n.value else {
            return Err(conflict().into());
        };
        if let Some(at) = n.retained_ordinals.first() {
            return Err(TextBodyUnresolved::RetainedContent {
                origin: origin.at(*at),
            }
            .into());
        }
        let part = match &origin {
            TextBodyOrigin::Object { object, .. } => object.part.len(),
            TextBodyOrigin::Theme { part, .. } => part.len(),
            _ => 0,
        };
        budget.bytes(
            part.checked_mul(20)
                .ok_or(PptxError::Limit("text body origin bytes"))?
                + 512,
        )?;
        for c in [
            &attributes.left_inset,
            &attributes.top_inset,
            &attributes.right_inset,
            &attributes.bottom_inset,
            &attributes.column_spacing,
        ]
        .into_iter()
        .flatten()
        {
            budget.bytes(c.lexical().len())?;
        }
        types::inherit(&mut self.values, attributes, &mut self.origins, &origin);
        for id in &n.children {
            budget.step()?;
            let c = node(catalog, *id)?;
            if !c.retained_ordinals.is_empty() {
                return Err(TextBodyUnresolved::RetainedContent {
                    origin: origin.at(c.retained_ordinals[0]),
                }
                .into());
            }
            if self.autofit.is_some() {
                continue;
            }
            let declared_by = origin.at(*id);
            self.autofit = Some(match c.element {
                NativeTextElement::NoAutofit => EffectiveTextAutofit::None { declared_by },
                NativeTextElement::SpAutoFit => EffectiveTextAutofit::Shape { declared_by },
                NativeTextElement::NormAutofit => {
                    let SourceTextValue::Autofit {
                        font_scale,
                        line_spacing_reduction,
                    } = &c.value
                    else {
                        return Err(conflict().into());
                    };
                    for v in [font_scale, line_spacing_reduction].into_iter().flatten() {
                        budget.bytes(v.lexical().len())?;
                    }
                    EffectiveTextAutofit::Normal {
                        declared_by,
                        font_scale: font_scale.clone().unwrap_or_else(|| {
                            NativePercentage::try_from("100000".to_owned())
                                .expect("profile percent")
                        }),
                        line_spacing_reduction: line_spacing_reduction.clone().unwrap_or_else(
                            || NativePercentage::try_from("0".to_owned()).expect("profile percent"),
                        ),
                        font_scale_defaulted: font_scale.is_none(),
                        line_spacing_reduction_defaulted: line_spacing_reduction.is_none(),
                    }
                }
                _ => return Err(conflict().into()),
            });
        }
        Ok(())
    }
    fn finish(mut self) -> EffectiveTextBody {
        types::inherit(
            &mut self.values,
            &types::defaults(),
            &mut self.origins,
            &TextBodyOrigin::ProfileDefault {},
        );
        EffectiveTextBody {
            attributes: self.values,
            origins: self.origins,
            autofit: self.autofit.unwrap_or(EffectiveTextAutofit::None {
                declared_by: TextBodyOrigin::ProfileDefault {},
            }),
        }
    }
}
struct Resolver<'a> {
    index: &'a SourceIndex,
    surface: &'a SourceSurface,
    objects: BTreeMap<(&'a str, u32), &'a SourceObject>,
}
impl Resolver<'_> {
    fn object(&self, reference: &SourceObjectRef) -> Result<&SourceObject, PptxError> {
        self.objects
            .get(&(reference.part.as_str(), reference.native_id))
            .copied()
            .ok_or_else(conflict)
    }
    fn merge_object(
        &self,
        p: &mut Partial,
        reference: &SourceObjectRef,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        let object = self.object(reference)?;
        if let Some(root) = object.text_body_ordinal {
            let catalog = &self
                .index
                .surfaces
                .get(&reference.part)
                .ok_or_else(conflict)?
                .text;
            if let Some(at) = node(catalog, root)?.retained_ordinals.first() {
                return Err(TextBodyUnresolved::RetainedContent {
                    origin: TextBodyOrigin::Object {
                        object: reference.clone(),
                        source_ordinal: *at,
                    },
                }
                .into());
            }
            let id =
                child(catalog, root, NativeTextElement::BodyPr, budget)?.ok_or_else(conflict)?;
            p.merge(
                catalog,
                id,
                TextBodyOrigin::Object {
                    object: reference.clone(),
                    source_ordinal: id,
                },
                budget,
            )?;
        }
        Ok(())
    }
    fn theme(&self, p: &mut Partial, budget: &mut Budget<'_>) -> Result<(), Failure> {
        let Some(reference) = &self.surface.effective_theme.base else {
            return Ok(());
        };
        let theme = self
            .index
            .themes
            .get(&reference.part)
            .ok_or_else(conflict)?;
        let Some(defaults) = &theme.text_defaults else {
            return Ok(());
        };
        for kind in [
            SourceThemeDefaultKind::Text,
            SourceThemeDefaultKind::Line,
            SourceThemeDefaultKind::Shape,
        ] {
            budget.step()?;
            if p.complete() {
                break;
            }
            let origin = TextBodyOrigin::Theme {
                part: reference.part.clone(),
                default_kind: Some(kind),
                source_ordinal: defaults.source_ordinal,
            };
            if let Some(at) = defaults.retained_ordinals.first() {
                return Err(TextBodyUnresolved::RetainedContent {
                    origin: TextBodyOrigin::Theme {
                        part: reference.part.clone(),
                        default_kind: None,
                        source_ordinal: *at,
                    },
                }
                .into());
            }
            let Some(entry) = defaults.entries.get(&kind) else {
                continue;
            };
            if let Some(at) = entry.retained_ordinals.first() {
                return Err(TextBodyUnresolved::RetainedContent {
                    origin: origin.at(*at),
                }
                .into());
            }
            for root in &entry.text.roots {
                budget.step()?;
                if node(&entry.text, root.source_ordinal)?.element == NativeTextElement::BodyPr {
                    p.merge(
                        &entry.text,
                        root.source_ordinal,
                        origin.at(root.source_ordinal),
                        budget,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn resolve(
        &self,
        target: SourceObjectRef,
        budget: &mut Budget<'_>,
    ) -> Result<EffectiveTextBody, Failure> {
        if self.object(&target)?.text_body_ordinal.is_none() {
            return Err(TextBodyUnresolved::NoTextBody {}.into());
        }
        let mut partial = Partial::default();
        let mut current = target;
        // Explicit local/layout placeholder declarations precede theme defaults;
        // the master fallback follows the documented theme txDef/lnDef/spDef chain.
        for hop in 0..3 {
            budget.step()?;
            let surface = self
                .index
                .surfaces
                .get(&current.part)
                .ok_or_else(conflict)?;
            if hop != 0 && surface.kind == SurfaceKind::Master {
                self.theme(&mut partial, budget)?;
                if partial.complete() {
                    return Ok(partial.finish());
                }
            }
            self.merge_object(&mut partial, &current, budget)?;
            if partial.complete() {
                return Ok(partial.finish());
            }
            match &self.object(&current)?.resolution.placeholder_match {
                SourcePlaceholderMatch::NotPlaceholder
                | SourcePlaceholderMatch::Detached
                | SourcePlaceholderMatch::Master => {
                    if !(hop != 0 && surface.kind == SurfaceKind::Master) {
                        self.theme(&mut partial, budget)?;
                    }
                    return Ok(partial.finish());
                }
                SourcePlaceholderMatch::Matched { target, .. } => {
                    let parent = self.index.surfaces.get(&target.part).ok_or_else(conflict)?;
                    if !matches!(
                        (surface.kind, parent.kind),
                        (SurfaceKind::Slide, SurfaceKind::Layout)
                            | (SurfaceKind::Layout, SurfaceKind::Master)
                    ) {
                        return Err(conflict().into());
                    }
                    current = target.clone();
                }
                matching => {
                    return Err(TextBodyUnresolved::Placeholder {
                        object: current.clone(),
                        matching: matching.clone(),
                    }
                    .into());
                }
            }
        }
        Err(conflict().into())
    }
}
pub fn query(
    index: &SourceIndex,
    request: &SourceTextBodyQuery,
    limits: TextBodyLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTextBodies, PptxError> {
    cancelled(check)?;
    if index.source_sha256 != request.expected_source_sha256 {
        return Err(PptxError::SourceConflict(
            "text body source digest differs".into(),
        ));
    }
    if request.objects.len() > limits.max_queries {
        return Err(PptxError::Limit("text body queries"));
    }
    let surface = index
        .surfaces
        .get(&request.surface)
        .ok_or_else(|| value("textBody.surface", "surface not in inspected source"))?;
    let mut budget = Budget {
        limits,
        check,
        steps: 0,
        bytes: 0,
    };
    let mut objects = BTreeMap::new();
    for (part, surface) in &index.surfaces {
        for object in &surface.objects {
            budget.step()?;
            objects.insert((part.as_str(), object.native_id), object);
        }
    }
    let resolver = Resolver {
        index,
        surface,
        objects,
    };
    let mut results = Vec::with_capacity(request.objects.len());
    for id in &request.objects {
        budget.bytes(request.surface.len() + 2048)?;
        if !resolver
            .objects
            .contains_key(&(request.surface.as_str(), *id))
        {
            return Err(value("textBody.objects", "object not in requested surface"));
        }
        let target = SourceObjectRef {
            part: request.surface.clone(),
            native_id: *id,
        };
        let outcome = match resolver.resolve(target, &mut budget) {
            Ok(body) => TextBodyOutcome::Resolved {
                body: Box::new(body),
            },
            Err(Failure::Unresolved(reason)) => TextBodyOutcome::Unresolved { reason },
            Err(Failure::Abort(error)) => return Err(error),
        };
        results.push(SourceTextBodyResult {
            native_id: *id,
            outcome,
        });
    }
    Ok(SourceTextBodies {
        source_sha256: index.source_sha256.clone(),
        surface: request.surface.clone(),
        profile: request.profile,
        objects: results,
    })
}
