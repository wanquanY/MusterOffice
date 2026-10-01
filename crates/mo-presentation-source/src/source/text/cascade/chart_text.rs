//! Genuine chart text scopes. Shared native character/paragraph inheritance is
//! reused without allocating shape objects or applying shape/master defaults.
use super::*;
use crate::source::charts::{SourceChartPart, SourceChartTextBody};
use NativeTextElement as N;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartTextCascade {
    pub chart_part: String,
    pub chart_sha256: mo_common::Digest,
    pub body_source_ordinal: u32,
    /// High to low priority, excluding the body itself. These are native txPr roots.
    pub property_roots: Vec<u32>,
    /// Declared cascade only. Chart style/default, paint, fonts and layout remain
    /// separate computations; None does not mean a shape default was selected.
    pub paragraphs: Vec<CascadedParagraph>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ChartTextOutcome {
    Cascaded { text: Box<ChartTextCascade> },
    Unresolved { reason: TextCascadeUnresolved },
}
/// Source inspection is the trust boundary. A resolver borrows that immutable
/// chart and shares budgets across all requested text bodies on the chart.
pub struct ChartTextResolver<'a> {
    chart: &'a SourceChartPart,
    bodies: BTreeMap<u32, &'a SourceChartTextBody>,
    budget: Budget<'a>,
    paragraphs: usize,
}
impl<'a> ChartTextResolver<'a> {
    /// The generated content style is the paragraph's defRPr cascade, never its
    /// endParaRPr. No fictitious a:r or object is introduced to obtain this style.
    pub fn default_character(
        &mut self,
        text: &ChartTextCascade,
        paragraph: u32,
    ) -> Result<Result<CascadedCharacterStyle, TextCascadeUnresolved>, PptxError> {
        match self.default_character_inner(text, paragraph) {
            Ok(style) => Ok(Ok(style)),
            Err(Failure::Unresolved(reason)) => Ok(Err(reason)),
            Err(Failure::Abort(e)) => Err(e),
        }
    }
    fn default_character_inner(
        &mut self,
        text: &ChartTextCascade,
        paragraph: u32,
    ) -> Result<CascadedCharacterStyle, Failure> {
        if text.chart_part != self.chart.part || text.chart_sha256 != self.chart.sha256 {
            return Err(conflict().into());
        }
        let body = *self
            .bodies
            .get(&text.body_source_ordinal)
            .ok_or_else(conflict)?;
        let p = body
            .paragraphs
            .get(paragraph as usize)
            .ok_or_else(conflict)?;
        if text
            .paragraphs
            .get(paragraph as usize)
            .map(|p| p.source_ordinal)
            != Some(p.source_ordinal)
        {
            return Err(conflict().into());
        }
        let mut at = root(self.chart, body, &mut self.budget)?;
        at.id = p.source_ordinal;
        at.origin = at.origin.at(p.source_ordinal);
        let mut paragraphs = vec![at];
        let mut seen = BTreeSet::from([body.source_ordinal]);
        for id in &text.property_roots {
            self.budget.step()?;
            if !seen.insert(*id) {
                return Err(conflict().into());
            }
            let body = *self.bodies.get(id).ok_or_else(conflict)?;
            let at = root(self.chart, body, &mut self.budget)?;
            if at.node()?.element != N::TxPr {
                return Err(conflict().into());
            }
            paragraphs.push(at.child(N::P, &mut self.budget)?.ok_or_else(conflict)?);
        }
        let mut characters = vec![];
        for p in paragraphs {
            p.checked(&mut self.budget)?;
            if let Some(ppr) = p.child(N::PPr, &mut self.budget)? {
                ppr.checked(&mut self.budget)?;
                if let Some(rpr) = ppr.child(N::DefRPr, &mut self.budget)? {
                    characters.push(Layer::Native(rpr));
                }
            }
        }
        native::character(&characters, false, &mut self.budget)
    }
    /// Generic DrawingML character defaults for label computation. Size and
    /// typeface must still resolve from the chart; shape defaults never enter.
    /// This does not implement chart-title, legend or chart-style role defaults.
    pub fn complete_label_style(
        &mut self,
        style: &mut CascadedCharacterStyle,
    ) -> Result<(), PptxError> {
        self.budget.step()?;
        let mut defaults = properties::character_defaults();
        defaults.size = None;
        properties::character(
            &mut style.attributes,
            &defaults,
            &mut style.origins,
            &TextStyleOrigin::ProfileDefault {},
            &mut self.budget,
        )
    }
    pub(in crate::source::text) fn is_bound_to(&self, chart: &SourceChartPart) -> bool {
        std::ptr::eq(self.chart, chart)
    }
    pub fn new(
        chart: &'a SourceChartPart,
        limits: TextCascadeLimits,
        check: &'a dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
            runs: 0,
        };
        let mut bodies = BTreeMap::new();
        for body in &chart.annotations.text_bodies {
            budget.step()?;
            budget.bytes(64)?;
            if bodies.insert(body.source_ordinal, body).is_some() {
                return Err(conflict());
            }
        }
        Ok(Self {
            chart,
            bodies,
            budget,
            paragraphs: 0,
        })
    }
    pub fn declaration(
        &self,
        reference: &TextStyleDeclaration,
    ) -> Result<&'a SourceTextNode, PptxError> {
        let TextStyleOrigin::Chart {
            part,
            body_source_ordinal,
            source_ordinal,
        } = &reference.origin
        else {
            return Err(conflict());
        };
        if part != &self.chart.part {
            return Err(conflict());
        }
        self.bodies
            .get(body_source_ordinal)
            .and_then(|b| b.styles.nodes.get(source_ordinal))
            .filter(|n| n.element == reference.element)
            .ok_or_else(conflict)
    }
    pub fn resolve(
        &mut self,
        body_source_ordinal: u32,
        property_roots: &[u32],
    ) -> Result<ChartTextOutcome, PptxError> {
        match self.resolve_inner(body_source_ordinal, property_roots) {
            Ok(text) => Ok(ChartTextOutcome::Cascaded {
                text: Box::new(text),
            }),
            Err(Failure::Unresolved(reason)) => Ok(ChartTextOutcome::Unresolved { reason }),
            Err(Failure::Abort(e)) => Err(e),
        }
    }
    fn resolve_inner(
        &mut self,
        body_source_ordinal: u32,
        property_roots: &[u32],
    ) -> Result<ChartTextCascade, Failure> {
        let body = *self.bodies.get(&body_source_ordinal).ok_or_else(conflict)?;
        let body_root = root(self.chart, body, &mut self.budget)?;
        let mut seen = BTreeSet::from([body_source_ordinal]);
        let mut inherited = vec![];
        for id in property_roots {
            self.budget.step()?;
            if !seen.insert(*id) {
                return Err(conflict().into());
            }
            let parent = *self.bodies.get(id).ok_or_else(conflict)?;
            let at = root_location(self.chart, parent);
            if at.node()?.element != N::TxPr {
                return Err(conflict().into());
            }
            let at = root(self.chart, parent, &mut self.budget)?;
            let p = at.child(N::P, &mut self.budget)?.ok_or_else(conflict)?;
            p.checked(&mut self.budget)?;
            if let Some(ppr) = p.child(N::PPr, &mut self.budget)? {
                inherited.push(Layer::Native(ppr));
            }
        }
        let mut paragraphs = vec![];
        for p in body_root.children(&mut self.budget)? {
            if p.node()?.element != N::P {
                continue;
            }
            self.paragraphs = self
                .paragraphs
                .checked_add(1)
                .filter(|n| *n <= self.budget.limits.max_paragraphs)
                .ok_or(PptxError::Limit("chart text cascade paragraphs"))?;
            p.checked(&mut self.budget)?;
            let content = body
                .paragraphs
                .get(paragraphs.len())
                .filter(|v| v.source_ordinal == p.id)
                .ok_or_else(conflict)?;
            let mut parents = p
                .child(N::PPr, &mut self.budget)?
                .into_iter()
                .map(Layer::Native)
                .collect::<Vec<_>>();
            parents.extend(inherited.iter().cloned());
            paragraphs.push(native::paragraph(
                &p,
                &parents,
                &|run| content.runs.get(run).map(|r| r.kind),
                false,
                &mut self.budget,
            )?);
        }
        if paragraphs.len() != body.paragraphs.len() || paragraphs.is_empty() {
            return Err(conflict().into());
        }
        self.budget
            .bytes(self.chart.part.len() + property_roots.len() * 4 + 128)?;
        Ok(ChartTextCascade {
            chart_part: self.chart.part.clone(),
            chart_sha256: self.chart.sha256.clone(),
            body_source_ordinal,
            property_roots: property_roots.to_vec(),
            paragraphs,
        })
    }
}
fn root_location<'a>(chart: &SourceChartPart, body: &'a SourceChartTextBody) -> Location<'a> {
    Location {
        catalog: &body.styles,
        id: body.source_ordinal,
        origin: TextStyleOrigin::Chart {
            part: chart.part.clone(),
            body_source_ordinal: body.source_ordinal,
            source_ordinal: body.source_ordinal,
        },
    }
}
fn root<'a>(
    chart: &SourceChartPart,
    body: &'a SourceChartTextBody,
    budget: &mut Budget<'_>,
) -> Result<Location<'a>, Failure> {
    budget.bytes(chart.part.len() + 96)?;
    let at = root_location(chart, body);
    let n = at.checked(budget)?;
    let root = body.styles.root(at.id).ok_or_else(conflict)?;
    if root.owner.is_some()
        || root.cell.is_some()
        || n.parent.is_some()
        || !matches!(n.element, N::TxPr | N::Rich)
    {
        return Err(conflict().into());
    }
    let reason = if let Some(list) = at.child(N::LstStyle, budget)? {
        list.checked(budget)?;
        (!list.node()?.children.is_empty()).then_some(ChartTextUnresolved::ListStyle)
    } else {
        None
    };
    let reason = reason.or_else(|| {
        if n.element != N::TxPr {
            None
        } else if body.paragraphs.len() != 1 {
            Some(ChartTextUnresolved::MultiplePropertyParagraphs)
        } else if !body.paragraphs[0].runs.is_empty() {
            Some(ChartTextUnresolved::PropertyTextRuns)
        } else {
            None
        }
    });
    if let Some(reason) = reason {
        return Err(TextCascadeUnresolved::ChartText {
            origin: at.origin,
            reason,
        }
        .into());
    }
    Ok(at)
}
