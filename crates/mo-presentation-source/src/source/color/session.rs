//! A batch shares bindings and budgets; native style colors stay at working precision.
use super::*;

#[derive(Clone, Copy)]
pub(in crate::source) struct ExpressionRef<'a> {
    pub value: &'a SourceColorValue,
    pub transforms: &'a [SourceColorTransform],
}

/// Deferred native context is consulted only when evaluation reaches phClr,
/// including through a theme color. No eager validation of unused references.
pub(in crate::source) enum Placeholder<'a> {
    Expression(Option<ExpressionRef<'a>>),
    Deferred(&'a mut dyn FnMut(&mut Budget<'_>) -> Computed<Option<ExpressionRef<'a>>>),
}
impl<'a> Placeholder<'a> {
    pub fn resolve(&mut self, budget: &mut Budget<'_>) -> Computed<Option<ExpressionRef<'a>>> {
        match self {
            Self::Expression(value) => Ok(*value),
            Self::Deferred(lookup) => lookup(budget),
        }
    }
}

pub(in crate::source) struct Evaluation {
    pub color: Result<Color, ColorUnresolved>,
    pub dependencies: Vec<ColorDependency>,
    pub notices: Vec<ColorNotice>,
}

pub(in crate::source) struct Session<'a> {
    map: Option<&'a SourceColorMap>,
    scheme: Option<(&'a str, &'a SourceColorScheme)>,
    context: &'a ColorContext,
    budget: Budget<'a>,
}
impl<'a> Session<'a> {
    pub fn new(
        index: &'a SourceIndex,
        surface: &'a super::super::SourceSurface,
        _profile: ColorProfile,
        context: &'a ColorContext,
        limits: ColorLimits,
        check: &'a dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        let mut session = Self {
            map: None,
            scheme: None,
            context,
            budget: Budget {
                limits,
                steps: 0,
                percentage_bytes: 0,
                check,
            },
        };
        session.select_surface(index, surface)?;
        Ok(session)
    }
    /// Overlay explicit bindings without resetting the cumulative expression budget.
    pub fn overlay(
        &mut self,
        map: Option<&'a SourceColorMap>,
        scheme: Option<(&'a str, &'a SourceColorScheme)>,
    ) {
        if let Some(map) = map {
            self.map = Some(map);
        }
        if let Some(scheme) = scheme {
            self.scheme = Some(scheme);
        }
    }
    /// Replace surface bindings without resetting the cumulative expression budget.
    pub fn select_surface(
        &mut self,
        index: &'a SourceIndex,
        surface: &'a super::super::SourceSurface,
    ) -> Result<(), PptxError> {
        let invalid =
            || PptxError::SourceConflict("color binding does not match inspected source".into());
        let map = surface
            .resolved_color_mapping
            .as_ref()
            .map(|binding| {
                match index
                    .surfaces
                    .get(&binding.part)
                    .and_then(|s| s.color_mapping.as_ref())
                {
                    Some(SourceColorMapping::Explicit {
                        source_ordinal,
                        mapping,
                    }) if *source_ordinal == binding.source_ordinal => Ok(mapping),
                    _ => Err(invalid()),
                }
            })
            .transpose()?;
        let scheme = surface
            .theme_selection
            .colors
            .as_ref()
            .map(|binding| {
                index
                    .themes
                    .get(&binding.part)
                    .and_then(|t| t.color_scheme.as_ref())
                    .filter(|s| s.source_ordinal == binding.source_ordinal)
                    .map(|scheme| (binding.part.as_str(), scheme))
                    .ok_or_else(invalid)
            })
            .transpose()?;
        self.map = map;
        self.scheme = scheme;
        Ok(())
    }
    fn run(
        &mut self,
        placeholder: Placeholder<'_>,
        evaluate: impl FnOnce(&mut Resolver<'_, '_, '_>) -> Computed<Color>,
    ) -> Result<Evaluation, PptxError> {
        let mut resolver = Resolver {
            map: self.map,
            scheme: self.scheme,
            context: self.context,
            budget: &mut self.budget,
            placeholder,
            in_placeholder: false,
            stack: Vec::new(),
            dependencies: Vec::new(),
            notices: Vec::new(),
        };
        let color = match evaluate(&mut resolver) {
            Ok(color) => Ok(color),
            Err(Failure::Unresolved(reason)) => Err(reason),
            Err(Failure::Abort(error)) => return Err(error),
        };
        Ok(Evaluation {
            color,
            dependencies: resolver.dependencies,
            notices: resolver.notices,
        })
    }
    pub fn scheme(&mut self, name: SchemeColor) -> Result<Evaluation, PptxError> {
        self.run(Placeholder::Expression(None), |resolver| {
            resolver.scheme(name)
        })
    }
    pub fn expression(
        &mut self,
        expression: ExpressionRef<'_>,
        placeholder: Option<ExpressionRef<'_>>,
    ) -> Result<Evaluation, PptxError> {
        self.expression_with_placeholder(expression, Placeholder::Expression(placeholder))
    }
    pub fn expression_with_placeholder(
        &mut self,
        expression: ExpressionRef<'_>,
        placeholder: Placeholder<'_>,
    ) -> Result<Evaluation, PptxError> {
        self.run(placeholder, |resolver| {
            resolver.expression(expression.value, expression.transforms)
        })
    }
}
