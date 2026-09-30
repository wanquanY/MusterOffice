//! The chart's font scheme can override the owning slide's scheme. No host
//! discovery, fake fontRef, implicit shape fallback or chart-style guessing.
use super::*;
use crate::source::charts::SourceChartPart;

pub struct ChartTypefaceContext<'a> {
    index: &'a SourceIndex,
    surface: &'a SourceSurface,
    chart: &'a SourceChartPart,
    overlay: Option<(&'a str, &'a theme::SourceThemePart)>,
}
impl<'a> ChartTypefaceContext<'a> {
    /// Inputs must come from the same inspected package. The chart binding and
    /// optional theme relationship are verified by the owning chart compiler.
    pub fn new(
        index: &'a SourceIndex,
        object: &SourceObjectRef,
        chart: &'a SourceChartPart,
        overlay: Option<(&'a str, &'a theme::SourceThemePart)>,
    ) -> Result<Self, PptxError> {
        let surface = index.surfaces.get(&object.part).ok_or_else(conflict)?;
        if !surface
            .objects
            .iter()
            .any(|o| o.native_id == object.native_id && o.kind == SourceObjectKind::GraphicFrame)
        {
            return Err(conflict());
        }
        if overlay.is_some_and(|(_, t)| t.kind != theme::SourceThemeKind::Override) {
            return Err(conflict());
        }
        Ok(Self {
            index,
            surface,
            chart,
            overlay,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        &self,
        resolver: &ChartTextResolver<'_>,
        text: &ChartTextCascade,
        paragraph: u32,
        run: Option<u32>,
        slot: NativeFontSlot,
        script: Option<&str>,
        limits: TypefaceLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TypefaceOutcome, PptxError> {
        cancelled(check)?;
        check_script(script)?;
        if !resolver.is_bound_to(self.chart)
            || text.chart_part != self.chart.part
            || text.chart_sha256 != self.chart.sha256
        {
            return Err(conflict());
        }
        let p = text
            .paragraphs
            .get(paragraph as usize)
            .ok_or_else(conflict)?;
        let style = match run {
            Some(r) => &p.runs.get(r as usize).ok_or_else(conflict)?.style,
            None => &p.end_style,
        };
        let Some(reference) = style.declarations.get(&slot.character()) else {
            return unresolved(TypefaceUnresolved::MissingDeclaration {});
        };
        let input = match selection::native(resolver.declaration(reference)?, reference, limits)? {
            Ok(input) => input,
            Err(reason) => return unresolved(reason),
        };
        resolve_input(input, reference, slot, script, limits, check, &|| {
            if let Some((part, t)) = self.overlay
                && let Some(scheme) = &t.font_scheme
            {
                return Ok(Some((
                    SourceThemeSchemeRef {
                        part: part.into(),
                        source_ordinal: scheme.source_ordinal,
                    },
                    scheme,
                )));
            }
            let Some(selection) = &self.surface.theme_selection.fonts else {
                return Ok(None);
            };
            let scheme = self
                .index
                .themes
                .get(&selection.part)
                .and_then(|t| t.font_scheme.as_ref())
                .filter(|s| s.source_ordinal == selection.source_ordinal)
                .ok_or_else(conflict)?;
            Ok(Some((selection.clone(), scheme)))
        })
    }
}
