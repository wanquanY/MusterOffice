//! Pure color arithmetic. Author declarations stay in their owning format/model.
//! Components are unassociated (not premultiplied); no host color or ICC state.
mod space;
pub use space::{linear_to_srgb, srgb_to_linear};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("non-finite color arithmetic input")]
pub struct NonFinite;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    components: Components,
    alpha: f64,
}

/// Retain the current working space. Converting after every transform would
/// erase hue at zero saturation and introduce needless transfer/rounding work.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Components {
    Srgb([f64; 3]),
    Linear([f64; 3]),
    Hsl([f64; 3]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrgbSample {
    pub rgba8: [u8; 4],
    pub rgba16: [u16; 4],
    pub clipped: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Adjustment {
    Set(f64),
    Offset(f64),
    Scale(f64),
}
impl Adjustment {
    fn apply(self, value: f64) -> Result<f64, NonFinite> {
        let parameter = match self {
            Self::Set(p) | Self::Offset(p) | Self::Scale(p) => p,
        };
        finite(parameter)?;
        Ok(match self {
            Self::Set(p) => p,
            Self::Offset(p) => value + p,
            Self::Scale(p) => value * p,
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub enum RgbChannel {
    Red,
    Green,
    Blue,
}
#[derive(Debug, Clone, Copy)]
pub enum HslChannel {
    Hue,
    Saturation,
    Luminance,
}

impl Color {
    pub fn srgb(rgb: [f64; 3], alpha: f64) -> Result<Self, NonFinite> {
        for v in rgb.into_iter().chain([alpha]) {
            finite(v)?;
        }
        Ok(Self {
            components: Components::Srgb(rgb),
            alpha: unit(alpha),
        })
    }
    pub fn srgb8(rgb: [u8; 3]) -> Self {
        Self {
            components: Components::Srgb(rgb.map(|v| f64::from(v) / 255.0)),
            alpha: 1.0,
        }
    }
    pub fn linear(rgb: [f64; 3], alpha: f64) -> Result<Self, NonFinite> {
        for v in rgb.into_iter().chain([alpha]) {
            finite(v)?;
        }
        Ok(Self {
            components: Components::Linear(rgb),
            alpha: unit(alpha),
        })
    }
    pub fn hsl(
        hue_turns: f64,
        saturation: f64,
        luminance: f64,
        alpha: f64,
    ) -> Result<Self, NonFinite> {
        for v in [hue_turns, saturation, luminance, alpha] {
            finite(v)?;
        }
        Ok(Self {
            components: Components::Hsl([wrap(hue_turns), unit(saturation), unit(luminance)]),
            alpha: unit(alpha),
        })
    }
    fn srgb_rgb(self) -> [f64; 3] {
        match self.components {
            Components::Srgb(rgb) => rgb,
            Components::Linear(rgb) => rgb.map(linear_to_srgb),
            Components::Hsl(hsl) => space::hsl_to_srgb(hsl),
        }
    }
    fn linear_rgb(self) -> [f64; 3] {
        match self.components {
            Components::Linear(rgb) => rgb,
            _ => self.srgb_rgb().map(srgb_to_linear),
        }
    }
    fn hsl_components(self) -> [f64; 3] {
        match self.components {
            Components::Hsl(hsl) => hsl,
            _ => space::srgb_to_hsl(self.srgb_rgb().map(unit)),
        }
    }
    /// Full working precision for a renderer. Quantization is a separate boundary.
    pub fn srgb_components(self) -> [f64; 4] {
        let [r, g, b] = self.srgb_rgb();
        [r, g, b, self.alpha]
    }
    pub fn linear_components(self) -> [f64; 4] {
        let [r, g, b] = self.linear_rgb();
        [r, g, b, self.alpha]
    }
    pub fn clipped_for_srgb(self) -> bool {
        self.srgb_rgb().iter().any(|v| !(0.0..=1.0).contains(v))
    }
    pub fn rgba8(self) -> [u8; 4] {
        self.srgb_components()
            .map(|v| libm::floor(unit(v) * 255.0 + 0.5) as u8)
    }
    pub fn rgba16(self) -> [u16; 4] {
        self.srgb_components()
            .map(|v| libm::floor(unit(v) * 65535.0 + 0.5) as u16)
    }
    /// Produce both diagnostic precisions with a single working-space conversion.
    /// Taking a sample never changes the color's working-space state.
    pub fn sample_srgb(self) -> SrgbSample {
        let rgba = self.srgb_components();
        SrgbSample {
            rgba8: rgba.map(|v| libm::floor(unit(v) * 255.0 + 0.5) as u8),
            rgba16: rgba.map(|v| libm::floor(unit(v) * 65535.0 + 0.5) as u16),
            clipped: rgba[..3].iter().any(|v| !(0.0..=1.0).contains(v)),
        }
    }
    pub fn alpha(&mut self, adjustment: Adjustment) -> Result<(), NonFinite> {
        self.alpha = unit(adjustment.apply(self.alpha)?);
        Ok(())
    }
    pub fn rgb_linear(
        &mut self,
        channel: RgbChannel,
        adjustment: Adjustment,
    ) -> Result<(), NonFinite> {
        let i = match channel {
            RgbChannel::Red => 0,
            RgbChannel::Green => 1,
            RgbChannel::Blue => 2,
        };
        let mut rgb = self.linear_rgb();
        rgb[i] = unit(adjustment.apply(rgb[i])?);
        self.components = Components::Linear(rgb);
        Ok(())
    }
    pub fn rgb_srgb(
        &mut self,
        channel: RgbChannel,
        adjustment: Adjustment,
    ) -> Result<(), NonFinite> {
        let i = match channel {
            RgbChannel::Red => 0,
            RgbChannel::Green => 1,
            RgbChannel::Blue => 2,
        };
        let mut rgb = self.srgb_rgb();
        rgb[i] = unit(adjustment.apply(rgb[i])?);
        self.components = Components::Srgb(rgb);
        Ok(())
    }
    /// Rotate hue cyclically (e.g. complement); ordinary channel edits clamp.
    pub fn rotate_hue(&mut self, turns: f64) -> Result<(), NonFinite> {
        finite(turns)?;
        let mut hsl = self.hsl_components();
        hsl[0] = wrap(hsl[0] + turns);
        self.components = Components::Hsl(hsl);
        Ok(())
    }
    pub fn hsl_adjust(
        &mut self,
        channel: HslChannel,
        adjustment: Adjustment,
    ) -> Result<(), NonFinite> {
        let mut hsl = self.hsl_components();
        let i = match channel {
            HslChannel::Hue => 0,
            HslChannel::Saturation => 1,
            HslChannel::Luminance => 2,
        };
        let value = adjustment.apply(hsl[i])?;
        finite(value)?;
        hsl[i] = unit(value);
        self.components = Components::Hsl(hsl);
        Ok(())
    }
    /// Linear-light mixing. A fraction of 1 retains the input, 0 uses white/black.
    pub fn tint_shade(&mut self, fraction: f64, white: bool) -> Result<(), NonFinite> {
        finite(fraction)?;
        let fraction = unit(fraction);
        self.components = Components::Linear(self.linear_rgb().map(|linear| {
            unit(if white {
                1.0 - (1.0 - linear) * fraction
            } else {
                linear * fraction
            })
        }));
        Ok(())
    }
    pub fn invert_linear(&mut self) {
        self.components = Components::Linear(self.linear_rgb().map(|v| unit(1.0 - v)));
    }
    pub fn gray_srgb(&mut self, weights: [f64; 3]) -> Result<(), NonFinite> {
        for v in weights {
            finite(v)?;
        }
        let rgb = self.srgb_rgb();
        let gray = rgb[0] * weights[0] + rgb[1] * weights[1] + rgb[2] * weights[2];
        finite(gray)?;
        self.components = Components::Srgb([unit(gray); 3]);
        Ok(())
    }
    /// Apply the transfer curve to linear-light components; keep representation.
    pub fn gamma(&mut self, inverse: bool) {
        self.components = Components::Linear(self.linear_rgb().map(|v| {
            let value = unit(v);
            if inverse {
                srgb_to_linear(value)
            } else {
                linear_to_srgb(value)
            }
        }));
    }
}

fn finite(value: f64) -> Result<(), NonFinite> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(NonFinite)
    }
}
fn unit(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}
fn wrap(v: f64) -> f64 {
    v - libm::floor(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srgb_transfer_and_sampling_do_not_round_working_channels() {
        let c = Color::linear([0.5, 0.0031308, 0.0], 0.5).unwrap();
        assert!((c.srgb_rgb()[0] - 0.7353569830524495).abs() < 1e-15);
        assert_eq!(c.rgba8(), [188, 10, 0, 128]);
        assert!((srgb_to_linear(0.5) - 0.21404114048223255).abs() < 1e-15);
        for value in 0..=255 {
            let a = Color::srgb8([value; 3]);
            assert_eq!(a.rgba8(), [value, value, value, 255]);
            assert_eq!(
                Color::linear(a.srgb_rgb().map(srgb_to_linear), 1.0)
                    .unwrap()
                    .rgba8(),
                a.rgba8()
            );
        }
    }
    #[test]
    fn hue_wrap_achromatic_and_transparent_colors_are_well_defined() {
        assert_eq!(
            Color::hsl(-1.0 / 3.0, 1.0, 0.5, 0.0).unwrap().rgba8(),
            [0, 0, 255, 0]
        );
        let mut c = Color::srgb8([255, 0, 0]);
        c.hsl_adjust(HslChannel::Luminance, Adjustment::Set(0.0))
            .unwrap();
        c.hsl_adjust(HslChannel::Luminance, Adjustment::Set(0.5))
            .unwrap();
        assert_eq!(c.rgba8(), [255, 0, 0, 255]);
        let mut c = Color::hsl(2.0 / 3.0, 0.0, 0.5, 1.0).unwrap();
        assert_eq!(c.rgba8(), [128, 128, 128, 255]);
        c.hsl_adjust(HslChannel::Saturation, Adjustment::Set(1.0))
            .unwrap();
        assert_eq!(c.rgba8(), [0, 0, 255, 255]);
        c.hsl_adjust(HslChannel::Hue, Adjustment::Offset(0.5))
            .unwrap();
        assert_eq!(c.rgba8(), [255, 0, 0, 255]);
        c.hsl_adjust(HslChannel::Hue, Adjustment::Offset(-0.25))
            .unwrap();
        assert_eq!(c.rgba8(), [128, 0, 255, 255]);
    }
    #[test]
    fn transform_order_and_linear_rgb_space_are_observable() {
        let mut a = Color::srgb8([64, 128, 192]);
        a.rgb_linear(RgbChannel::Red, Adjustment::Set(0.5)).unwrap();
        assert_eq!(a.rgba8(), [188, 128, 192, 255]);
        let mut b = a;
        a.alpha(Adjustment::Scale(0.5)).unwrap();
        a.alpha(Adjustment::Offset(0.5)).unwrap();
        b.alpha(Adjustment::Offset(0.5)).unwrap();
        b.alpha(Adjustment::Scale(0.5)).unwrap();
        assert_ne!(a.rgba16(), b.rgba16());
    }
    #[test]
    fn invalid_numbers_fail_and_extended_gamut_is_reported_at_sampling() {
        assert!(Color::linear([f64::NAN, 0.0, 0.0], 1.0).is_err());
        let c = Color::linear([-0.5, 1.5, 0.0], 1.0).unwrap();
        assert!(c.clipped_for_srgb());
        assert_eq!(c.rgba8(), [0, 255, 0, 255]);
        assert!(c.linear_components()[0] < 0.0);
    }
}
