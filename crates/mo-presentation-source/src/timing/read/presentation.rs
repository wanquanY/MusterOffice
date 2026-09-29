//! Validate editorial roles without flattening their clocks or identities.
//! Their original XML remains in the source package for lossless edits.
use super::*;
use crate::timing::preset::presentation_preset;
use mo_timeline::{ContainerKind, PresentationRole, PresentationTrigger};

pub(super) fn validate(
    _tree: &[Node],
    common: &Node,
    kind: ContainerKind,
    list: Option<&Node>,
) -> Result<Option<PresentationRole>, PptxError> {
    let has_preset = ["presetID", "presetClass", "presetSubtype"]
        .iter()
        .any(|name| common.element.attribute(name).is_some());
    match common.element.attribute("nodeType") {
        None if !has_preset => Ok(None),
        Some("mainSeq") if !has_preset => {
            if kind != ContainerKind::Sequence {
                return Err(unsupported("main sequence requires a sequence container"));
            }
            Ok(Some(PresentationRole::MainSequence))
        }
        Some(role @ ("withEffect" | "afterEffect" | "clickEffect"))
            if kind == ContainerKind::Parallel =>
        {
            let list = list.ok_or_else(|| unsupported("effect without behavior list"))?;
            if list.children.is_empty() || list.children.len() > 2 {
                return Err(unsupported("preset behavior count"));
            }
            // The compiled time graph validates the actual typed payload against
            // this identity; a visible Set cannot masquerade as Disappear.
            let preset = presentation_preset(
                integer(common, "presetID")?,
                common.element.attribute("presetClass").unwrap_or(""),
            )
            .ok_or_else(|| unsupported("unmapped presentation preset"))?;
            value(common, "presetSubtype", "0")?;
            Ok(Some(PresentationRole::Effect {
                preset,
                trigger: match role {
                    "clickEffect" => PresentationTrigger::Click,
                    "withEffect" => PresentationTrigger::WithPrevious,
                    _ => PresentationTrigger::AfterPrevious,
                },
            }))
        }
        _ => Err(unsupported("presentation container role or preset")),
    }
}
