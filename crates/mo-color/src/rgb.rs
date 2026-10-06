//! Explicit D65 RGB transfer/primary conversion. No ICC, device or implicit tone mapping.
mod matrix;
mod transfer;
pub use matrix::{Matrix3, Primaries};
pub use transfer::Transfer;

/// Relative linear light uses BT.2408 diffuse white as one. SDR white is aligned
/// to it; PQ is absolute, and HLG requires an explicit reference display.
pub const REFERENCE_WHITE_NITS: f64 = 203.0;
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum RgbError {
    #[error("non-finite RGB input or result")]
    NonFinite,
    #[error("invalid color parameter")]
    Parameter,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Encoding {
    pub primaries: Primaries,
    pub transfer: Transfer,
}
#[derive(Debug, Clone, Copy)]
pub struct LinearSample {
    pub rgb: [f64; 3],
    pub signal_clipped: bool,
}
#[derive(Debug, Clone, Copy)]
pub struct RgbTransform {
    source: Encoding,
    matrix: Matrix3,
}
impl RgbTransform {
    pub fn new(source: Encoding, target: Primaries) -> Result<Self, RgbError> {
        source.transfer.validate()?;
        Ok(Self {
            source,
            matrix: source.primaries.to(target),
        })
    }
    pub fn is_hdr(&self) -> bool {
        self.source.transfer.is_hdr()
    }
    pub fn decode(&self, rgb: [f64; 3]) -> Result<LinearSample, RgbError> {
        finite(rgb)?;
        let (linear, signal_clipped) = self
            .source
            .transfer
            .decode(rgb, self.source.primaries.luminance())?;
        let rgb = self.matrix.apply(linear);
        finite(rgb)?;
        Ok(LinearSample {
            rgb,
            signal_clipped,
        })
    }
}
pub(super) fn finite(v: [f64; 3]) -> Result<(), RgbError> {
    if v.iter().all(|n| n.is_finite()) {
        Ok(())
    } else {
        Err(RgbError::NonFinite)
    }
}
