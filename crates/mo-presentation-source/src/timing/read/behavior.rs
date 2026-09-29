//! Behavior-specific payloads. Scheduling, target binding and lifecycle are
//! read once by the parent module. Unmapped payloads retain the original OPC.
use super::*;
use mo_timeline::{Effect, ScaleValue, Visibility};
mod motion;
mod rotation;
mod scale;

pub(super) enum Payload {
    Rotation {
        from: i32,
        to: i32,
        composition: mo_timeline::RotationComposition,
    },
    Scale {
        from: ScaleValue,
        to: ScaleValue,
    },
    Visibility(Visibility),
    Fade(mo_timeline::FadeTransition),
    MotionPath(mo_timeline::MotionPath),
    MotionLine {
        from: mo_timeline::MotionPoint,
        to: mo_timeline::MotionPoint,
    },
}
impl Payload {
    /// Native defaults depend on the behavior's value domain. In particular,
    /// the admitted layout-relative motion path supplies a complete offset
    /// from the original layout center; `base` (including omission) does not
    /// accumulate the preceding path's offset. Lower that native profile to
    /// the existing absolute motion channel. Do not generalize this to
    /// numeric property animations, whose underlying-value rules differ.
    pub fn common_attributes(&self, behavior: &Node) -> Result<(), PptxError> {
        attrs(behavior, &["additive", "accumulate", "xfrmType"])?;
        match (self, behavior.element.attribute("additive")) {
            (_, Some("repl"))
            | (Self::Rotation { .. }, None | Some("base"))
            | (Self::MotionLine { .. } | Self::MotionPath(_), None | Some("base"))
            | (Self::Visibility(_) | Self::Fade(_), None) => (),
            (_, mode) => {
                return Err(unsupported(format!(
                    "cBhvr additive mode {} for this behavior",
                    mode.unwrap_or("(native default)")
                )));
            }
        }
        if behavior.element.attribute("accumulate").is_some() {
            value(behavior, "accumulate", "none")?;
        }
        if behavior.element.attribute("xfrmType").is_some() {
            value(behavior, "xfrmType", "pt")?;
        }
        Ok(())
    }
    pub fn effect(self, target: ObjectId) -> Effect {
        match self {
            Self::MotionPath(path) => Effect::MotionPath { target, path },
            Self::MotionLine { from, to } => Effect::MotionLine { target, from, to },
            Self::Rotation {
                from,
                to,
                composition,
            } => Effect::Rotation {
                target,
                from,
                to,
                composition,
            },
            Self::Scale { from, to } => Effect::Scale { target, from, to },
            Self::Visibility(value) => Effect::SetVisibility { target, value },
            Self::Fade(transition) => Effect::Fade { target, transition },
        }
    }
    pub fn properties(&self, tree: &[Node], list: &Node) -> Result<(), PptxError> {
        attrs(list, &[])?;
        let names: &[&str] = match self {
            Self::Rotation { .. } => &["r"],
            Self::Scale { .. } => &["ScaleX", "ScaleY"],
            Self::Visibility(_) => &["style.visibility"],
            Self::Fade(_) => &[],
            Self::MotionLine { .. } | Self::MotionPath(_) => &["ppt_x", "ppt_y"],
        };
        let properties = children(tree, list, &vec!["attrName"; names.len()])?;
        for (property, expected) in properties.iter().zip(names) {
            attrs(property, &[])?;
            if !property.children.is_empty() || property.text.trim() != *expected {
                return Err(unsupported("transform property names"));
            }
        }
        Ok(())
    }
}
pub(super) fn read<'a>(tree: &'a [Node], anim: &Node) -> Result<(Payload, &'a Node), PptxError> {
    if anim.element.name.is(P, "animRot") {
        rotation::read(tree, anim)
    } else if anim.element.name.is(P, "animMotion") {
        motion::read(tree, anim)
    } else if anim.element.name.is(P, "animScale") {
        scale::read(tree, anim)
    } else if anim.element.name.is(P, "animEffect") {
        attrs(anim, &["filter", "transition"])?;
        value(anim, "filter", "fade")?;
        let transition = match anim.element.attribute("transition") {
            Some("in") => mo_timeline::FadeTransition::In,
            Some("out") => mo_timeline::FadeTransition::Out,
            _ => return Err(unsupported("fade transition")),
        };
        Ok((Payload::Fade(transition), single(tree, anim, "cBhvr")?))
    } else if anim.element.name.is(P, "set") {
        attrs(anim, &[])?;
        let parts = children(tree, anim, &["cBhvr", "to"])?;
        attrs(parts[1], &[])?;
        let value = single(tree, parts[1], "strVal")?;
        attrs(value, &["val"])?;
        empty(tree, value)?;
        let value = match value.element.attribute("val") {
            Some("visible") => Visibility::Visible,
            Some("hidden") => Visibility::Hidden,
            _ => return Err(unsupported("visibility set value")),
        };
        Ok((Payload::Visibility(value), parts[0]))
    } else {
        Err(unsupported(format!("behavior {}", anim.element.name.local)))
    }
}
