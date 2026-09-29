//! Native time conditions. Node edges require an explicit `tn`; target events
//! on shapes, runtime-child selectors and missing references are never guessed.
use super::*;

/// MS-OI29500 2.1.1156: PowerPoint uses zero when `delay` is absent.
/// An explicit invalid value is not a default and must still be rejected.
pub(super) fn delay(condition: &Node) -> Result<RationalTime, PptxError> {
    Ok(ms(if condition.element.attribute("delay").is_some() {
        integer(condition, "delay")?
    } else {
        0
    }))
}

pub(super) fn read(
    tree: &[Node],
    condition: &Node,
    known_objects: &BTreeSet<u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
) -> Result<mo_timeline::TimeCondition, PptxError> {
    attrs(condition, &["evt", "delay"])?;
    if condition.element.attribute("delay") == Some("indefinite") {
        // Only the explicit untriggered form is mapped. Do not discard event
        // declarations or targets attached to other indefinite conditions.
        attrs(condition, &["delay"])?;
        empty(tree, condition)?;
        return Ok(TimeCondition::Never {});
    }
    let delay = delay(condition)?;
    Ok(match condition.element.attribute("evt") {
        None => {
            empty(tree, condition)?;
            TimeCondition::At { offset: delay }
        }
        Some(event @ ("begin" | "end" | "onBegin" | "onEnd")) => {
            let target = single(tree, condition, "tn")?;
            attrs(target, &["val"])?;
            empty(tree, target)?;
            TimeCondition::After {
                node: n_id(integer(target, "val")?),
                event: match event {
                    "begin" => NodeEvent::Begin,
                    "end" => NodeEvent::End,
                    "onBegin" => NodeEvent::OnBegin,
                    "onEnd" => NodeEvent::OnEnd,
                    _ => unreachable!("matched node event"),
                },
                delay,
            }
        }
        Some(event @ ("onClick" | "onNext" | "onPrev")) => {
            let target = single(tree, condition, "tgtEl")?;
            attrs(target, &[])?;
            let slide_target =
                target.children.len() == 1 && tree[target.children[0]].element.name.is(P, "sldTgt");
            let target = if slide_target {
                let s = single(tree, target, "sldTgt")?;
                attrs(s, &[])?;
                empty(tree, s)?;
                None
            } else {
                Some(object(tree, target, known_objects, object_bindings)?)
            };
            if event == "onClick" {
                TimeCondition::Click { target, delay }
            } else {
                TimeCondition::Navigation {
                    direction: if event == "onNext" {
                        mo_timeline::NavigationDirection::Next
                    } else {
                        mo_timeline::NavigationDirection::Previous
                    },
                    target,
                    delay,
                }
            }
        }
        Some(_) => return Err(unsupported("condition event")),
    })
}
