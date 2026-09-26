use mo_geometry::{Affine, Point};
use mo_presentation_source::source::fill::resolve::FillTarget;
use mo_raster::{FillPath, ImageBrush};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageFillClip {
    /// Already rebased local path. Apply affine once, independently of the
    /// painted shape path, then intersect both filled regions.
    pub path: FillPath,
    pub affine: Affine,
    /// Nonnegative Q32 WORLD EMU bounds. Add to downstream scene/float error;
    /// this field must not be discarded when constructing a shared ClipNode.
    pub upstream_error: Point,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeImagePaint {
    pub profile: String,
    pub target: FillTarget,
    /// World paint with upstream errors included. A scene must not apply its
    /// path's transform to this brush a second time.
    pub brush: ImageBrush,
    /// Stretch rectangle is a hard clip, never simulated by decal filtering.
    /// Absence for tile does not remove the painted shape's own filled boundary.
    pub fill_clip: Option<ImageFillClip>,
}
#[derive(Debug, thiserror::Error)]
pub enum ImagePaintError {
    #[error("invalid native image paint: {0}")]
    Invalid(&'static str),
    #[error("native stationary image orientation policy required")]
    OrientationRequired,
    #[error("native image paint outside Q32 range")]
    Range,
    #[error("native image paint input uncertainty collapses its domain")]
    Precision,
    #[error("native image paint cancelled")]
    Cancelled,
}
