use super::*;
use mo_pptx::source::{
    color::ColorSample,
    fill::colors::FillPaintColors,
    line::{colors::LinePaintColor, resolve::*, *},
};
use mo_raster::{StrokeCap, StrokeJoin, StrokeStyle};

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
pub(super) fn fill(
    result: &mo_pptx::source::fill::colors::SourceFillColorResult,
    at: &SourcePageLocation,
    images_enabled: bool,
    placement: Option<&NativePlacement>,
    page_size: mo_presentation_model::Size,
    radial: Option<&crate::radial_layout::NativeRadialLayout>,
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
            let gradient = super::gradient::compile(result, placement, page_size, radial, check)
                .map_err(|e| e.at(at))?;
            Ok(Some(FillPaint::Gradient(Box::new(gradient))))
        }
        FillPaintColors::ImageResourcesRequired {} if images_enabled => Ok(Some(FillPaint::Image)),
        _ => Err(mapping(at, SourcePageIssue::Fill {})),
    }
}
pub(super) fn line(
    result: &mo_pptx::source::line::colors::SourceLineColorResult,
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
    if line.compound.value != NativeCompoundLine::Single
        || line.alignment.value != NativePenAlignment::Center
        || !matches!(line.dash, EffectiveLineDash::Preset { ref value, .. } if value.value == NativePresetDash::Solid)
        || line.head.kind.value != NativeLineEnd::None
        || line.tail.kind.value != NativeLineEnd::None
    {
        return Err(mapping(at, SourcePageIssue::Line {}));
    }
    let join = match line.join {
        EffectiveLineJoin::Round { .. } => StrokeJoin::Round {},
        EffectiveLineJoin::Bevel { .. } => StrokeJoin::Bevel {},
        // Native miter policy still needs its own end-to-end connection.
        EffectiveLineJoin::Miter { .. } => return Err(mapping(at, SourcePageIssue::Line {})),
    };
    Ok(Some((
        rgba8,
        StrokeStyle {
            width: Fixed::emu(line.width.value),
            join,
            cap: match line.cap.value {
                NativeLineCap::Flat => StrokeCap::Butt,
                NativeLineCap::Round => StrokeCap::Round,
                NativeLineCap::Square => StrokeCap::Square,
            },
        },
    )))
}

pub(super) fn background_redirect(result: &SourceFillColorResult, at: &SourcePageLocation) -> bool {
    at.object.is_some()
        && matches!(&result.style, FillOutcome::Resolved { redirects, .. }
            if redirects.iter().any(|r| matches!(r.target.target, FillTarget::Background {})))
}
