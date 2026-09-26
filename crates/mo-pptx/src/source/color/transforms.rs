use super::{Budget, Computed};
use crate::source::theme::SourceColorTransform;
use mo_color::{Adjustment as A, Color, HslChannel as H, RgbChannel as R};

pub(super) fn apply(
    color: &mut Color,
    transform: &SourceColorTransform,
    budget: &mut Budget<'_>,
) -> Computed<()> {
    use SourceColorTransform::*;
    match transform {
        Tint(p) => color.tint_shade(budget.percentage(p)?, true)?,
        Shade(p) => color.tint_shade(budget.percentage(p)?, false)?,
        Complement => color.rotate_hue(0.5)?,
        Inverse => color.invert_linear(),
        Gray => color.gray_srgb([0.22, 0.72, 0.06])?,
        Alpha(p) => color.alpha(A::Set(budget.percentage(p)?))?,
        AlphaOff(p) => color.alpha(A::Offset(budget.percentage(p)?))?,
        AlphaMod(p) => color.alpha(A::Scale(budget.percentage(p)?))?,
        Hue(a) => color.hsl_adjust(H::Hue, A::Set(f64::from(*a) / 21_600_000.0))?,
        HueOff(a) => color.hsl_adjust(H::Hue, A::Offset(f64::from(*a) / 21_600_000.0))?,
        HueMod(p) => color.hsl_adjust(H::Hue, A::Scale(budget.percentage(p)?))?,
        Sat(p) => color.hsl_adjust(H::Saturation, A::Set(budget.percentage(p)?))?,
        SatOff(p) => color.hsl_adjust(H::Saturation, A::Offset(budget.percentage(p)?))?,
        SatMod(p) => color.hsl_adjust(H::Saturation, A::Scale(budget.percentage(p)?))?,
        Lum(p) => color.hsl_adjust(H::Luminance, A::Set(budget.percentage(p)?))?,
        LumOff(p) => color.hsl_adjust(H::Luminance, A::Offset(budget.percentage(p)?))?,
        LumMod(p) => color.hsl_adjust(H::Luminance, A::Scale(budget.percentage(p)?))?,
        Red(p) => color.rgb_srgb(R::Red, A::Set(budget.percentage(p)?))?,
        RedOff(p) => color.rgb_srgb(R::Red, A::Offset(budget.percentage(p)?))?,
        RedMod(p) => color.rgb_srgb(R::Red, A::Scale(budget.percentage(p)?))?,
        Green(p) => color.rgb_srgb(R::Green, A::Set(budget.percentage(p)?))?,
        GreenOff(p) => color.rgb_srgb(R::Green, A::Offset(budget.percentage(p)?))?,
        GreenMod(p) => color.rgb_srgb(R::Green, A::Scale(budget.percentage(p)?))?,
        Blue(p) => color.rgb_srgb(R::Blue, A::Set(budget.percentage(p)?))?,
        BlueOff(p) => color.rgb_srgb(R::Blue, A::Offset(budget.percentage(p)?))?,
        BlueMod(p) => color.rgb_srgb(R::Blue, A::Scale(budget.percentage(p)?))?,
        Gamma => color.gamma(false),
        InvGamma => color.gamma(true),
    }
    Ok(())
}
