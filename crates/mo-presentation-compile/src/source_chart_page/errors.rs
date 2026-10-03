use super::*;
pub(super) fn format(
    part: &str,
    ordinal: u32,
    error: mo_charts::number_format::FormatError,
) -> SourcePageError {
    use mo_charts::number_format::FormatError as E;
    match error {
        E::Cancelled => RasterError::Cancelled.into(),
        E::Limit(s) => RasterError::Limit(s).into(),
        e => invalid(part, ordinal, e.to_string()),
    }
}
pub(super) fn sector(
    part: &str,
    ordinal: u32,
    error: mo_charts::sectors::SectorError,
) -> SourcePageError {
    use mo_charts::sectors::SectorError as E;
    match error {
        E::Cancelled => RasterError::Cancelled.into(),
        E::Limit(s) => RasterError::Limit(s).into(),
        e => invalid(part, ordinal, e.to_string()),
    }
}
pub(super) fn circular(
    part: &str,
    ordinal: u32,
    error: crate::source_chart::SourceCircularError,
) -> SourcePageError {
    use crate::{chart_geometry::ChartGeometryError as G, source_chart::SourceCircularError as E};
    match error {
        E::Source(e) => e.into(),
        E::Geometry(G::Cancelled) => RasterError::Cancelled.into(),
        E::Geometry(G::Limit(s)) => RasterError::Limit(s).into(),
        E::Geometry(G::Sectors(e)) => sector(part, ordinal, e),
        E::Geometry(G::Precision) => RasterError::Precision.into(),
        E::Geometry(G::Range) => RasterError::Range.into(),
        e => invalid(part, ordinal, e.to_string()),
    }
}
