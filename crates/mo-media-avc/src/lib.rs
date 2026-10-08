//! Bounded AVC parameter-set and picture-header interpretation, shared by native
//! and WASM. This is not a decoder or proof of full bitstream conformance. The
//! caller supplies complete access-unit boundaries and immutable byte identity.
mod bits;
mod inspector;
mod read;
mod sps;
pub use inspector::AvcInspector;
use mo_media::{ErrorKind, MediaError};
pub use read::ReadBudget;
use serde::Serialize;
pub type Result<T> = std::result::Result<T, MediaError>;
pub(crate) fn invalid(message: &'static str) -> MediaError {
    MediaError::new(ErrorKind::Invalid, message)
}
pub(crate) fn unsupported(message: &'static str) -> MediaError {
    MediaError::new(ErrorKind::Unsupported, message)
}
pub(crate) fn limit(message: &'static str) -> MediaError {
    MediaError::new(ErrorKind::Limit, message)
}
pub(crate) fn active(check: &dyn Fn() -> bool) -> Result<()> {
    let cancelled =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)).map_err(|_| {
            MediaError::new(ErrorKind::CallbackPanic, "AVC cancellation callback panic")
        })?;
    if cancelled {
        Err(MediaError::new(
            ErrorKind::Cancelled,
            "AVC inspection cancelled",
        ))
    } else {
        Ok(())
    }
}
pub const MAX_PARAMETER_BYTES: usize = 65535;
pub const MAX_ACTIVE_PARAMETERS: usize = 1 << 20;
pub const MAX_NALS_PER_ACCESS_UNIT: u32 = 4096;

/// Raw VUI values with their signalling provenance. Unspecified colour values
/// stay 2; the host must not turn these into an inferred Rec.709 declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VuiColor {
    pub signal_present: bool,
    pub description_present: bool,
    pub full_range: bool,
    pub primaries: u8,
    pub transfer: u8,
    pub matrix: u8,
}
impl Default for VuiColor {
    fn default() -> Self {
        Self {
            signal_present: false,
            description_present: false,
            full_range: false,
            primaries: 2,
            transfer: 2,
            matrix: 2,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub id: u8,
    pub profile: u8,
    pub level: u8,
    pub chroma_format: u8,
    pub bit_depth_luma: u8,
    pub bit_depth_chroma: u8,
    pub coded_width: u32,
    pub coded_height: u32,
    /// x, y, width, height in coded luma samples, before orientation/SAR.
    pub visible: [u32; 4],
    pub frame_mbs_only: bool,
    pub separate_colour_plane: bool,
    pub max_references: u8,
    /// None means no explicit restriction, not a zero-sized decoder buffer.
    pub reorder_frames: Option<u8>,
    pub decoder_buffer_frames: Option<u8>,
    pub color: VuiColor,
    /// H.264 E.2.1 codes, only meaningful for chroma_format == 1.
    pub chroma_location_top: u8,
    pub chroma_location_bottom: u8,
    pub chroma_location_present: bool,
}
impl Sequence {
    /// Progressive frame reconstruction only. Other field interpretations need
    /// a field-aware render contract, never the top-field value applied to both.
    pub fn progressive_chroma_location(&self) -> Result<Option<&'static str>> {
        if !self.frame_mbs_only || self.separate_colour_plane {
            return Err(unsupported("field or separate-plane chroma reconstruction"));
        }
        match self.chroma_format {
            0 | 3 => Ok(None),
            2 => Ok(Some("left")), // Clause 6.2: horizontally co-sited 4:2:2.
            1 if self.chroma_location_top == self.chroma_location_bottom => {
                ["left", "center", "topLeft", "top", "bottomLeft", "bottom"]
                    .get(self.chroma_location_top as usize)
                    .copied()
                    .map(Some)
                    .ok_or_else(|| invalid("chroma location"))
            }
            _ => Err(unsupported("unequal field chroma locations")),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Picture {
    pub pps_id: u8,
    /// Changes only when that active SPS's bytes change, not on unrelated SPSs.
    pub sps_revision: u32,
    pub pps_revision: u32,
    pub idr: bool,
    pub field_picture: bool,
    pub bottom_field: bool,
    pub sequence: Sequence,
}
