//! Effects use the same physical placeholder hierarchy and consuming style
//! context as fills. Declarations are selected atomically; an explicit empty
//! effect list or idx=0 blocks more distant effects. No source XML is changed.
use super::*;
use crate::{
    PptxError, cancelled,
    source::{
        color::{self, ExpressionRef, Placeholder, Session},
        *,
    },
};
use std::collections::BTreeMap;

pub struct Selected<'a> {
    pub part: &'a str,
    pub properties: &'a SourceEffectProperties,
    pub nodes: &'a BTreeMap<u32, SourceEffectNode>,
    placeholder: Option<&'a SourceColor>,
}
pub struct Resolver<'a> {
    index: &'a SourceIndex,
    view: &'a SourceSurface,
    objects: BTreeMap<(&'a str, u32), &'a SourceObject>,
    colors: Session<'a>,
    check: &'a dyn Fn() -> bool,
}
fn invalid(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}
fn unsupported(message: &str) -> PptxError {
    PptxError::Unsupported(message.into())
}
impl<'a> Resolver<'a> {
    pub fn new(
        index: &'a SourceIndex,
        surface: &str,
        context: &'a color::ColorContext,
        check: &'a dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        let view = index
            .surfaces
            .get(surface)
            .ok_or_else(|| invalid("effect drawing surface"))?;
        let mut objects = BTreeMap::new();
        for (part, s) in &index.surfaces {
            for o in &s.objects {
                cancelled(check)?;
                if objects.len() >= 65_536 {
                    return Err(PptxError::Limit("effect object bindings"));
                }
                if objects.insert((part.as_str(), o.native_id), o).is_some() {
                    return Err(invalid("duplicate effect object"));
                }
            }
        }
        Ok(Self {
            index,
            view,
            objects,
            colors: Session::new(
                index,
                view,
                color::ColorProfile::Ecma3762016DraftV1,
                context,
                color::ColorLimits {
                    max_queries: 8192,
                    ..Default::default()
                },
                check,
            )?,
            check,
        })
    }
    pub fn select(&self, part: &str, id: u32) -> Result<Option<Selected<'a>>, PptxError> {
        let mut part = self
            .index
            .surfaces
            .get_key_value(part)
            .map(|(part, _)| part.as_str())
            .ok_or_else(|| invalid("effect surface binding"))?;
        let mut object = *self
            .objects
            .get(&(part, id))
            .ok_or_else(|| invalid("effect object binding"))?;
        for _ in 0..3 {
            cancelled(self.check)?;
            let surface = self
                .index
                .surfaces
                .get(part)
                .ok_or_else(|| invalid("effect surface binding"))?;
            if let Some(properties) = &object.effects {
                return Ok(Some(Selected {
                    part,
                    properties,
                    nodes: &surface.effect_nodes,
                    placeholder: object
                        .effect_reference
                        .as_ref()
                        .and_then(|r| r.color.as_ref()),
                }));
            }
            if let Some(reference) = &object.effect_reference {
                if !reference.retained_ordinals.is_empty() {
                    return Err(unsupported("retained effect reference properties"));
                }
                if reference.index == 0 {
                    return Ok(None);
                }
                let binding = self
                    .view
                    .theme_selection
                    .format
                    .as_ref()
                    .ok_or_else(|| unsupported("missing effect format scheme"))?;
                let theme = self
                    .index
                    .themes
                    .get(&binding.part)
                    .ok_or_else(|| invalid("effect theme binding"))?;
                let scheme = theme
                    .format_scheme
                    .as_ref()
                    .filter(|s| s.source_ordinal == binding.source_ordinal)
                    .ok_or_else(|| invalid("effect format scheme binding"))?;
                let style = scheme
                    .effects
                    .get((reference.index - 1) as usize)
                    .and_then(|e| e.effect_style.as_ref())
                    .ok_or_else(|| unsupported("effect style index out of range"))?;
                if !style.retained_ordinals.is_empty() {
                    return Err(unsupported("retained effect style or 3D properties"));
                }
                return Ok(Some(Selected {
                    part: &binding.part,
                    properties: &style.effects,
                    nodes: &theme.effect_nodes,
                    placeholder: reference.color.as_ref(),
                }));
            }
            match &object.resolution.placeholder_match {
                SourcePlaceholderMatch::Matched { target, .. } => {
                    let (parent, kind) = match surface.kind {
                        SurfaceKind::Slide => (&surface.links.layout, SurfaceKind::Layout),
                        SurfaceKind::Layout => (&surface.links.master, SurfaceKind::Master),
                        SurfaceKind::Master => {
                            return Err(invalid("master effect placeholder parent"));
                        }
                    };
                    if parent.as_deref() != Some(target.part.as_str())
                        || !self
                            .index
                            .surfaces
                            .get(&target.part)
                            .is_some_and(|s| s.kind == kind)
                    {
                        return Err(invalid("effect placeholder hierarchy"));
                    }
                    part = &target.part;
                    object = *self
                        .objects
                        .get(&(part, target.native_id))
                        .ok_or_else(|| invalid("effect placeholder binding"))?;
                }
                SourcePlaceholderMatch::NotPlaceholder
                | SourcePlaceholderMatch::Master
                | SourcePlaceholderMatch::Detached => return Ok(None),
                _ => return Err(unsupported("unresolved effect placeholder")),
            }
        }
        Err(invalid("effect placeholder depth"))
    }
    pub fn color(
        &mut self,
        selected: &Selected<'a>,
        value: &SourceColor,
    ) -> Result<color::ColorSample, PptxError> {
        let placeholder = selected.placeholder.map(|p| ExpressionRef {
            value: &p.value,
            transforms: &p.transforms,
        });
        let computed = self.colors.expression_with_placeholder(
            ExpressionRef {
                value: &value.value,
                transforms: &value.transforms,
            },
            Placeholder::Expression(placeholder),
        )?;
        Ok(color::ColorSample::from_computed(computed.color))
    }
}
