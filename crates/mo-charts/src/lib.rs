//! Bounded chart data and layout computation. No workbook, file, renderer, font,
//! host state or UI dependency. Format adapters own cache/source authority.
mod number;
pub mod sectors;
pub use number::{DecimalNumber, NumberError};
