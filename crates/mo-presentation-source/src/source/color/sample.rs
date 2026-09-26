//! Working channels for render consumers; display samples are diagnostics only.
use super::ColorUnresolved;
use mo_color::Color;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ColorSample {
    Resolved {
        /// Unassociated, unclipped working-precision channels.
        srgb: [f64; 4],
        linear: [f64; 4],
        /// Never feed these quantized samples back into native expressions.
        rgba8: [u8; 4],
        rgba16: [u16; 4],
        clipped_for_srgb: bool,
    },
    Unresolved {
        reason: ColorUnresolved,
    },
}
impl ColorSample {
    pub(in crate::source) fn from_computed(value: Result<Color, ColorUnresolved>) -> Self {
        match value {
            Ok(color) => {
                let srgb = color.srgb_components();
                let linear = color.linear_components();
                if !srgb.iter().chain(&linear).all(|v| v.is_finite()) {
                    return Self::Unresolved {
                        reason: ColorUnresolved::NumericRange,
                    };
                }
                let sample = color.sample_srgb();
                Self::Resolved {
                    srgb,
                    linear,
                    rgba8: sample.rgba8,
                    rgba16: sample.rgba16,
                    clipped_for_srgb: sample.clipped,
                }
            }
            Err(reason) => Self::Unresolved { reason },
        }
    }
}
