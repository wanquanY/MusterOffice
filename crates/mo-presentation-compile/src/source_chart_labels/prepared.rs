use super::*;

/// A group's point identities are indexed separately. Do not scan every sibling
/// dLbl again when resolving the inherited styles of each requested point.
pub(super) struct Declaration<'a> {
    pub node: &'a SourceChartAnnotation,
    pub markup: Vec<&'a SourceChartMarkup>,
    pub requires_series_parent: bool,
}
impl<'a> Declaration<'a> {
    pub fn new(
        node: &'a SourceChartAnnotation,
        budget: &Budget<'_>,
    ) -> Result<Self, ChartLabelError> {
        let mut requires_series_parent = node.number_format.is_some()
            || node.declarations.properties.iter().any(|p| {
                matches!(
                    p.kind,
                    ChartPropertyKind::Delete | ChartPropertyKind::ShowLeaderLines
                )
            });
        let mut markup = Vec::new();
        for item in &node.declarations.markup {
            budget.check()?;
            requires_series_parent |= matches!(
                item.kind,
                ChartMarkupKind::DataLabel
                    | ChartMarkupKind::ShapeProperties
                    | ChartMarkupKind::TextProperties
                    | ChartMarkupKind::LeaderLines
            );
            if item.kind != ChartMarkupKind::DataLabel {
                markup.push(item);
            }
        }
        Ok(Self {
            node,
            markup,
            requires_series_parent,
        })
    }
}
