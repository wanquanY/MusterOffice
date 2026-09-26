//! Shared native DrawingML declarations, independent of where they are used.
mod color;
mod coordinate;
mod font;
pub use font::SourceTextFont;
pub(super) use font::text_font;
mod names;
pub use color::{NativePercentage, SourceColor, SourceColorTransform, SourceColorValue};
pub(super) use color::{
    any_percentage, color, enumeration, fixed_percentage, positive_fixed_percentage,
    positive_percentage, required, transform,
};
pub use coordinate::NativeCoordinate;
pub use names::{ColorSlot, PresetColor, SchemeColor, SystemColor};
