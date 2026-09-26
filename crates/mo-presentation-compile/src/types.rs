use mo_common::{Digest, ObjectId, SlideId};
use mo_geometry::{Affine, Fixed, Point};
use mo_presentation_model::{ContainerId, Document, Size};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePlacementRequest {
    pub document: Document,
    pub slide: SlideId,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AffineUncertainty {
    /// Nonnegative outward bounds, raw dimensionless Q32.
    pub linear: [Fixed; 4],
    /// Nonnegative outward bounds, raw Q32 EMU.
    pub translation: Point,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectPlacement {
    pub object: ObjectId,
    pub parent: ContainerId,
    pub depth: u32,
    /// Shape/picture/connector bounds are local 0..size; custom paths and groups
    /// use their own viewport. The current author model has no child offset.
    pub source_size: Size,
    /// Subtract this exact source center before applying the evaluated affine.
    pub anchor: Point,
    /// Already evaluated into page space; do not apply parent placements again.
    pub affine: Affine,
    /// Covers author scale/angle computation and final Q32 quantization. It is
    /// additional to mo-render's bounds for an already-quantized affine graph.
    pub uncertainty: AffineUncertainty,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlacementSurface {
    pub container: ContainerId,
    /// Stable author order, groups followed by their descendants. A group entry
    /// is a coordinate-space record, not a paint instruction.
    pub objects: Vec<ObjectPlacement>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePlacements {
    pub profile: String,
    pub document_sha256: Digest,
    pub slide: SlideId,
    pub hidden: bool,
    pub page_size: Size,
    /// Master, layout, slide contexts; not a resolved paint or placeholder list.
    pub surfaces: Vec<PlacementSurface>,
    pub unique_angles: u32,
}
