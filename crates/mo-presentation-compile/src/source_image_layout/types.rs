use mo_geometry::{Fixed, Point};
use mo_image::{PhysicalPixelSize, PixelExtent};
use mo_raster::ImageTile;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Coordinates are interpreted by the containing field, never implicitly EMU.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageLayoutRectangle {
    pub left: Fixed,
    pub top: Fixed,
    pub right: Fixed,
    pub bottom: Fixed,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "source", rename_all = "camelCase", deny_unknown_fields)]
pub enum ImageLayoutDensity {
    /// Stretch uses source pixels and target dimensions; no physical DPI needed.
    NotRequired,
    Drawingml {
        dpi: u32,
    },
    EncodedMetadata {
        x: PixelExtent,
        y: PixelExtent,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageLayoutUncertainty {
    /// Outward per-edge errors, left/top/right/bottom, in Q32 SOURCE pixels.
    pub source_rectangle: [Fixed; 4],
    /// Outward per-edge errors in Q32 local shape EMU.
    pub fill_rectangle: [Fixed; 4],
    pub origin: Point,
    /// Errors in Q32 EMU per source pixel, before shape placement.
    pub pixel_step: Point,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeImageLayout {
    pub profile: String,
    /// Cropping is interpreted on the already oriented, normalized image axes.
    pub source_rectangle: ImageLayoutRectangle,
    /// Stretch destination or first tile, before shape/group orientation.
    pub fill_rectangle: ImageLayoutRectangle,
    /// Position of source boundary (0,0) in the unrotated local shape box.
    pub origin: Point,
    /// Signed local EMU per normalized source pixel, x/y axes respectively.
    pub pixel_step: Point,
    pub tile_x: ImageTile,
    pub tile_y: ImageTile,
    /// Stretch additionally requires a hard clip to fill_rectangle. The shape
    /// path itself always clips. Decal sampling is not a replacement for this.
    pub clip_to_fill_rectangle: bool,
    /// Placement must consume this property; this local result is not a world
    /// ImageBrush and must not be used as one by merely adding shape origin.
    pub rotate_with_shape: bool,
    pub density: ImageLayoutDensity,
    pub uncertainty: ImageLayoutUncertainty,
}
#[derive(Debug, thiserror::Error)]
pub enum ImageLayoutError {
    #[error("invalid native image layout: {0}")]
    Invalid(&'static str),
    #[error("native image layout source number exceeds lexical budget")]
    LexicalLimit,
    #[error("native image layout numeric range")]
    Range,
    #[error("native image layout precision")]
    Precision,
    #[error("native tile requires physical pixel size: {0:?}")]
    PhysicalSize(PhysicalPixelSize),
    #[error("native image layout cancelled")]
    Cancelled,
}
