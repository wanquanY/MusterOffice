use super::*;
use crate::source_stroke::{StrokeInput, stroke};
use mo_presentation_source::source::{
    color::ColorSample,
    fill::colors::FillPaintColors,
    line::{colors::LinePaintColor, resolve::*},
};
use mo_raster::StrokeStyle;

#[derive(Clone)]
pub(crate) enum FillPaint {
    Solid([u8; 4]),
    Image,
    Gradient(Box<mo_raster::Gradient>),
    Background,
}
impl FillPaint {
    pub(super) fn gradient_stops(&self) -> usize {
        match self {
            Self::Gradient(g) => g.stops.len(),
            _ => 0,
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn fill(
    result: &mo_presentation_source::source::fill::colors::SourceFillColorResult,
    at: &SourcePageLocation,
    images_enabled: bool,
    placement: Option<&NativePlacement>,
    page_size: mo_presentation_model::Size,
    radial: Option<&crate::radial_layout::NativeRadialLayout>,
    region: Option<&SourcePaintRegion>,
    check: &dyn Fn() -> bool,
) -> Result<Option<FillPaint>, SourcePageError> {
    // Background windows consume the already resolved page background. Their
    // own box/color context must not re-layout or re-composite that background.
    if background_redirect(result, at) {
        return Ok(Some(FillPaint::Background));
    }
    match &result.colors {
        FillPaintColors::None {} => Ok(None),
        FillPaintColors::Solid { color } => match color.outcome {
            ColorSample::Resolved { rgba8, .. } => Ok(Some(FillPaint::Solid(rgba8))),
            _ => Err(mapping(at, SourcePageIssue::Fill {})),
        },
        FillPaintColors::Gradient { .. } => {
            let gradient =
                super::gradient::compile(result, placement, page_size, radial, region, check)
                    .map_err(|e| e.at(at))?;
            Ok(Some(FillPaint::Gradient(Box::new(gradient))))
        }
        FillPaintColors::ImageResourcesRequired {} if images_enabled => Ok(Some(FillPaint::Image)),
        _ => Err(mapping(at, SourcePageIssue::Fill {})),
    }
}
pub(super) fn line(
    result: &mo_presentation_source::source::line::colors::SourceLineColorResult,
    at: &SourcePageLocation,
) -> Result<Option<([u8; 4], StrokeStyle)>, SourcePageError> {
    if matches!(result.paint, LinePaintColor::None {}) {
        return Ok(None);
    }
    let LinePaintColor::Solid {
        outcome: ColorSample::Resolved { rgba8, .. },
        ..
    } = result.paint
    else {
        return Err(mapping(at, SourcePageIssue::Line {}));
    };
    let LineOutcome::Resolved { line } = &result.style else {
        return Err(mapping(at, SourcePageIssue::Line {}));
    };
    Ok(Some((
        rgba8,
        stroke(StrokeInput::from(line.as_ref()))
            .map_err(|_| mapping(at, SourcePageIssue::Line {}))?,
    )))
}

pub(super) fn table_stroke(
    geometry: &EffectiveLineGeometry,
    at: &SourcePageLocation,
) -> Result<StrokeStyle, SourcePageError> {
    stroke(StrokeInput::from(geometry)).map_err(|_| mapping(at, SourcePageIssue::Line {}))
}

pub(super) fn background_redirect(result: &SourceFillColorResult, at: &SourcePageLocation) -> bool {
    at.object.is_some()
        && matches!(&result.style, FillOutcome::Resolved { redirects, .. }
            if redirects.iter().any(|r| matches!(r.target.target, FillTarget::Background {})))
}
