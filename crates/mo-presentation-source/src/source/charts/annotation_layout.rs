//! Annotation-specific grammar entries used by the shared chart declaration reader.
use super::*;
use ChartAnnotationKind as N;
pub(super) fn property(kind: N, name: &str) -> Option<ChartPropertyKind> {
    use ChartPropertyKind::*;
    Some(match (kind, name) {
        (N::DataLabels | N::DataLabel | N::LegendEntry, "delete") => Delete,
        (N::DataLabels | N::DataLabel, "dLblPos") => LabelPosition,
        (N::DataLabels | N::DataLabel, "showLegendKey") => ShowLegendKey,
        (N::DataLabels | N::DataLabel, "showVal") => ShowValue,
        (N::DataLabels | N::DataLabel, "showCatName") => ShowCategoryName,
        (N::DataLabels | N::DataLabel, "showSerName") => ShowSeriesName,
        (N::DataLabels | N::DataLabel, "showPercent") => ShowPercent,
        (N::DataLabels | N::DataLabel, "showBubbleSize") => ShowBubbleSize,
        (N::DataLabels | N::DataLabel, "separator") => Separator,
        (N::DataLabels, "showLeaderLines") => ShowLeaderLines,
        (N::Legend, "legendPos") => LegendPosition,
        (N::Legend | N::Title, "overlay") => Overlay,
        (N::ManualLayout, "layoutTarget") => LayoutTarget,
        (N::ManualLayout, "xMode") => XMode,
        (N::ManualLayout, "yMode") => YMode,
        (N::ManualLayout, "wMode") => WidthMode,
        (N::ManualLayout, "hMode") => HeightMode,
        (N::ManualLayout, "x") => X,
        (N::ManualLayout, "y") => Y,
        (N::ManualLayout, "w") => Width,
        (N::ManualLayout, "h") => Height,
        _ => return None,
    })
}
pub(super) fn markup(kind: N, name: &str) -> Option<ChartMarkupKind> {
    use ChartMarkupKind::*;
    Some(match (kind, name) {
        (N::DataLabels | N::DataLabel | N::Legend | N::Title, "spPr") => ShapeProperties,
        (N::DataLabels | N::DataLabel | N::Legend | N::LegendEntry | N::Title, "txPr") => {
            TextProperties
        }
        (N::DataLabel | N::Legend | N::Title, "layout") => Layout,
        (N::Layout, "manualLayout") => ManualLayout,
        (N::DataLabel | N::Title, "tx") => TextSource,
        (N::DataLabels, "dLbl") => DataLabel,
        (N::Legend, "legendEntry") => LegendEntry,
        (N::DataLabels, "leaderLines") => LeaderLines,
        (_, "extLst") => Extensions,
        _ => return None,
    })
}
pub(super) fn structural(kind: N, name: &str) -> bool {
    matches!(
        (kind, name),
        (N::DataLabel | N::LegendEntry, "idx") | (N::DataLabels | N::DataLabel, "numFmt")
    )
}
