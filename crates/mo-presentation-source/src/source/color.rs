//! On-demand source color evaluation. Never mutates author declarations and never
//! reads host colors. Profile and source digest are explicit at the query boundary.
mod preset;
mod sample;
mod session;
pub use sample::ColorSample;
pub(in crate::source) use session::{ExpressionRef, Placeholder, Session};
mod transforms;
mod types;
use super::{SourceColorMap, SourceColorMapping, SourceIndex, theme::*};
use crate::{PptxError, cancelled};
use mo_color::Color;
pub use types::*;

pub(in crate::source) enum Failure {
    Unresolved(ColorUnresolved),
    Abort(PptxError),
}
impl From<PptxError> for Failure {
    fn from(error: PptxError) -> Self {
        Self::Abort(error)
    }
}
impl From<mo_color::NonFinite> for Failure {
    fn from(_: mo_color::NonFinite) -> Self {
        Self::Unresolved(ColorUnresolved::NumericRange)
    }
}
pub(in crate::source) type Computed<T> = Result<T, Failure>;

pub(in crate::source) struct Budget<'a> {
    limits: ColorLimits,
    steps: usize,
    percentage_bytes: usize,
    check: &'a dyn Fn() -> bool,
}
impl Budget<'_> {
    pub(in crate::source) fn is_cancelled(&self) -> bool {
        (self.check)()
    }
    pub(in crate::source) fn step(&mut self) -> Computed<()> {
        cancelled(self.check)?;
        if self.steps >= self.limits.max_steps {
            return Err(PptxError::Limit("color evaluation steps").into());
        }
        self.steps += 1;
        Ok(())
    }
    fn percentage(&mut self, value: &NativePercentage) -> Computed<f64> {
        let lexical = value.lexical();
        if lexical.len()
            > self
                .limits
                .max_percentage_bytes
                .saturating_sub(self.percentage_bytes)
        {
            return Err(PptxError::Limit("color percentage bytes").into());
        }
        self.percentage_bytes += lexical.len();
        let (number, divisor) = lexical
            .strip_suffix('%')
            .map_or((lexical, 100_000.0), |n| (n, 100.0));
        let number: f64 = number
            .parse()
            .map_err(|_| Failure::Unresolved(ColorUnresolved::NumericRange))?;
        if !number.is_finite() {
            return Err(Failure::Unresolved(ColorUnresolved::NumericRange));
        }
        Ok(number / divisor)
    }
}

struct Resolver<'a, 'b, 'c> {
    placeholder: Placeholder<'c>,
    in_placeholder: bool,
    map: Option<&'a SourceColorMap>,
    scheme: Option<(&'a str, &'a SourceColorScheme)>,
    context: &'a ColorContext,
    budget: &'b mut Budget<'a>,
    stack: Vec<ColorSlot>,
    dependencies: Vec<ColorDependency>,
    notices: Vec<ColorNotice>,
}

impl Resolver<'_, '_, '_> {
    fn scheme(&mut self, name: SchemeColor) -> Computed<Color> {
        self.budget.step()?;
        if name == SchemeColor::PhClr {
            if !self.in_placeholder
                && let Some(expression) = self.placeholder.resolve(self.budget)?
            {
                self.dependencies.push(ColorDependency::Placeholder);
                self.in_placeholder = true;
                let result = self.expression(expression.value, expression.transforms);
                self.in_placeholder = false;
                return result;
            }
            let rgba = self
                .context
                .placeholder
                .ok_or(Failure::Unresolved(ColorUnresolved::MissingPlaceholder))?;
            self.dependencies.push(ColorDependency::Placeholder);
            return Ok(Color::srgb(
                rgba[..3]
                    .try_into()
                    .map(|rgb: [u8; 3]| rgb.map(|v| f64::from(v) / 255.0))
                    .expect("three channels"),
                f64::from(rgba[3]) / 255.0,
            )?);
        }
        let slot = self.slot(name)?;
        if self.stack.contains(&slot) {
            return Err(Failure::Unresolved(ColorUnresolved::SchemeCycle { slot }));
        }
        let (part, scheme) = self
            .scheme
            .ok_or(Failure::Unresolved(ColorUnresolved::MissingColorScheme))?;
        let color = scheme.colors.get(&slot).ok_or(Failure::Unresolved(
            ColorUnresolved::MissingThemeSlot { slot },
        ))?;
        self.dependencies.push(ColorDependency::Theme {
            part: part.into(),
            slot,
            source_ordinal: color.source_ordinal,
        });
        // There are exactly 12 possible slots; cycle rejection bounds recursion.
        self.stack.push(slot);
        let result = self.color(color);
        self.stack.pop();
        result
    }
    fn slot(&self, name: SchemeColor) -> Computed<ColorSlot> {
        use SchemeColor::*;
        match name {
            Dk1 => return Ok(ColorSlot::Dk1),
            Lt1 => return Ok(ColorSlot::Lt1),
            Dk2 => return Ok(ColorSlot::Dk2),
            Lt2 => return Ok(ColorSlot::Lt2),
            _ => (),
        }
        let map = self
            .map
            .ok_or(Failure::Unresolved(ColorUnresolved::MissingColorMap))?;
        Ok(match name {
            Bg1 => map.bg1,
            Tx1 => map.tx1,
            Bg2 => map.bg2,
            Tx2 => map.tx2,
            Accent1 => map.accent1,
            Accent2 => map.accent2,
            Accent3 => map.accent3,
            Accent4 => map.accent4,
            Accent5 => map.accent5,
            Accent6 => map.accent6,
            Hlink => map.hlink,
            FolHlink => map.fol_hlink,
            PhClr | Dk1 | Lt1 | Dk2 | Lt2 => unreachable!("handled before map lookup"),
        })
    }
    fn color(&mut self, source: &SourceColor) -> Computed<Color> {
        self.expression(&source.value, &source.transforms)
    }
    fn expression(
        &mut self,
        value: &SourceColorValue,
        transforms: &[SourceColorTransform],
    ) -> Computed<Color> {
        use SourceColorValue::*;
        self.budget.step()?;
        let mut color = match value {
            Srgb { rgb } => Color::srgb8(*rgb),
            ScRgb { red, green, blue } => Color::linear(
                [
                    self.budget.percentage(red)?,
                    self.budget.percentage(green)?,
                    self.budget.percentage(blue)?,
                ],
                1.0,
            )?,
            Hsl {
                hue,
                saturation,
                luminance,
            } => Color::hsl(
                f64::from(*hue) / 21_600_000.0,
                self.budget.percentage(saturation)?,
                self.budget.percentage(luminance)?,
                1.0,
            )?,
            System { color, last_color } => {
                let (rgb, origin) = if let Some(rgb) = self.context.system_colors.get(color) {
                    (*rgb, SystemColorOrigin::HostContext)
                } else if let Some(rgb) = last_color {
                    (*rgb, SystemColorOrigin::FileLastColor)
                } else {
                    return Err(Failure::Unresolved(ColorUnresolved::MissingSystemColor {
                        color: *color,
                    }));
                };
                self.dependencies.push(ColorDependency::System {
                    color: *color,
                    origin,
                });
                Color::srgb8(rgb)
            }
            Scheme { slot } => self.scheme(*slot)?,
            Preset { color } => {
                if *color == PresetColor::LtGoldenrodYellow {
                    self.notice(ColorNotice::PresetAliasDiscrepancy);
                }
                Color::srgb8(preset::rgb(*color))
            }
        };
        for transform in transforms {
            self.budget.step()?;
            if matches!(transform, SourceColorTransform::Gray) {
                self.notice(ColorNotice::GrayWeightsProvisional);
            }
            transforms::apply(&mut color, transform, self.budget)?;
        }
        Ok(color)
    }
    fn notice(&mut self, notice: ColorNotice) {
        if !self.notices.contains(&notice) {
            self.notices.push(notice);
        }
    }
}

/// Uses a previously inspected immutable source. Native/WASM wire entry points
/// inspect the actual source themselves; they do not accept a caller-made index.
pub fn query(
    index: &SourceIndex,
    request: &SourceColorQuery,
    limits: ColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceColorPalette, PptxError> {
    cancelled(check)?;
    if index.source_sha256 != request.expected_source_sha256 {
        return Err(PptxError::SourceConflict(
            "color query source digest differs".into(),
        ));
    }
    if request.colors.len() > limits.max_queries {
        return Err(PptxError::Limit("color queries"));
    }
    let surface = index.surfaces.get(&request.surface).ok_or_else(|| {
        crate::value(
            "colorQuery.surface",
            "surface is not in the inspected source",
        )
    })?;
    let mut session = Session::new(
        index,
        surface,
        request.profile,
        &request.context,
        limits,
        check,
    )?;
    let mut colors = Vec::with_capacity(request.colors.len());
    for name in &request.colors {
        let evaluation = session.scheme(*name)?;
        let outcome = match evaluation.color {
            Ok(color) => {
                let sample = color.sample_srgb();
                ColorOutcome::Resolved {
                    rgba8: sample.rgba8,
                    rgba16: sample.rgba16,
                    clipped_for_srgb: sample.clipped,
                }
            }
            Err(reason) => ColorOutcome::Unresolved { reason },
        };
        colors.push(SchemeColorResult {
            scheme: *name,
            outcome,
            dependencies: evaluation.dependencies,
            notices: evaluation.notices,
        });
    }
    Ok(SourceColorPalette {
        source_sha256: index.source_sha256.clone(),
        surface: request.surface.clone(),
        profile: request.profile,
        color_mapping: surface.resolved_color_mapping.clone(),
        color_scheme: surface.theme_selection.colors.clone(),
        colors,
    })
}
