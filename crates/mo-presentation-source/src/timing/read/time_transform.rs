//! Reuse DrawingML percentage grammar and refuse lossy timing projection.
use super::*;
use crate::source::drawingml::NativePercentage;
use mo_timeline::TimeTransform;

pub(super) fn read(common: &Node) -> Result<Option<TimeTransform>, PptxError> {
    if ["spd", "autoRev", "accel", "decel"]
        .iter()
        .all(|a| common.element.attribute(a).is_none())
    {
        return Ok(None);
    }
    let percentage = |name: &str, fixed: bool, default: i32| -> Result<i32, PptxError> {
        let Some(value) = common.element.attribute(name) else {
            return Ok(default);
        };
        let parsed = if fixed {
            NativePercentage::positive_fixed(value)?
        } else {
            NativePercentage::try_from(value.trim().to_owned())
                .map_err(|e| crate::value("timing/timeTransform", e))?
        };
        parsed.thousandths().ok_or_else(|| {
            unsupported(format!(
                "{name} is not exactly representable as signed thousandths of a percent"
            ))
        })
    };
    let auto_reverse = match common.element.attribute("autoRev").map(str::trim) {
        None | Some("false" | "0") => false,
        Some("true" | "1") => true,
        _ => return Err(crate::value("timing/autoRev", "invalid XML boolean")),
    };
    Ok(Some(TimeTransform {
        speed_milli_percent: percentage("spd", false, 100_000)?,
        auto_reverse,
        acceleration_milli_percent: percentage("accel", true, 0)? as u32,
        deceleration_milli_percent: percentage("decel", true, 0)? as u32,
    }))
}
