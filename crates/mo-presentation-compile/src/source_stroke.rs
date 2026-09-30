//! Shared source stroke admission for shapes, tables and chart paths.
use mo_geometry::Fixed;
use mo_presentation_source::source::line::{resolve::*, *};
use mo_raster::{StrokeCap, StrokeJoin, StrokeStyle};

pub(crate) struct StrokeInput<'a> {
    width: mo_common::Emu,
    cap: NativeLineCap,
    compound: NativeCompoundLine,
    alignment: NativePenAlignment,
    dash: &'a EffectiveLineDash,
    join: &'a EffectiveLineJoin,
    head: NativeLineEnd,
    tail: NativeLineEnd,
}
impl<'a> From<&'a EffectiveLine> for StrokeInput<'a> {
    fn from(v: &'a EffectiveLine) -> Self {
        Self {
            width: v.width.value,
            cap: v.cap.value,
            compound: v.compound.value,
            alignment: v.alignment.value,
            dash: &v.dash,
            join: &v.join,
            head: v.head.kind.value,
            tail: v.tail.kind.value,
        }
    }
}
impl<'a> From<&'a EffectiveLineGeometry> for StrokeInput<'a> {
    fn from(v: &'a EffectiveLineGeometry) -> Self {
        Self {
            width: v.width.value,
            cap: v.cap.value,
            compound: v.compound.value,
            alignment: v.alignment.value,
            dash: &v.dash,
            join: &v.join,
            head: v.head.kind.value,
            tail: v.tail.kind.value,
        }
    }
}
pub(crate) fn stroke(line: StrokeInput<'_>) -> Result<StrokeStyle, &'static str> {
    if line.compound != NativeCompoundLine::Single
        || line.alignment != NativePenAlignment::Center
        || !matches!(line.dash, EffectiveLineDash::Preset {value,..} if value.value==NativePresetDash::Solid)
        || line.head != NativeLineEnd::None
        || line.tail != NativeLineEnd::None
    {
        return Err("unsupported native compound, alignment, dash or line ends");
    }
    let join = match line.join {
        EffectiveLineJoin::Round { .. } => StrokeJoin::Round {},
        EffectiveLineJoin::Bevel { .. } => StrokeJoin::Bevel {},
        EffectiveLineJoin::Miter { .. } => return Err("native miter stroke not admitted"),
    };
    Ok(StrokeStyle {
        width: Fixed::emu(line.width),
        join,
        cap: match line.cap {
            NativeLineCap::Flat => StrokeCap::Butt,
            NativeLineCap::Round => StrokeCap::Round,
            NativeLineCap::Square => StrokeCap::Square,
        },
    })
}
