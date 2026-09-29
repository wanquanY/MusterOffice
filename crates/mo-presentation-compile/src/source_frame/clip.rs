use super::*;
use mo_presentation_source::source::text::{
    NativeTextHorizontalOverflow, NativeTextVerticalOverflow,
};
use serde::{Deserialize, Serialize};
/// Axis-specific native overflow constraints, before object/group placement.
/// Unpainted glyphs and capacity measurements remain complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameClip {
    pub bounds: Rect,
    pub horizontal: bool,
    pub vertical: bool,
    pub conversion_error_bound: Fixed,
}
impl FrameClip {
    pub(super) fn from_body(body: &EffectiveTextBody, region: &SourceFrameRegion) -> Option<Self> {
        let horizontal =
            body.attributes.horizontal_overflow == Some(NativeTextHorizontalOverflow::Clip);
        let vertical = body.attributes.vertical_overflow == Some(NativeTextVerticalOverflow::Clip);
        (horizontal || vertical).then_some(Self {
            bounds: region.outer,
            horizontal,
            vertical,
            conversion_error_bound: region.conversion_error_bound,
        })
    }
}
