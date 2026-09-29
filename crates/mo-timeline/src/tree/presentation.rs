//! Editorial identity is part of presentation computation, not a host UI hint.
//! Initial visibility is a lower layer than active/frozen property assignments.
use super::*;
use mo_common::ObjectId;
use std::collections::BTreeSet;

pub(super) fn compile(
    timeline: &Timeline,
    hierarchy: &Hierarchy,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<ObjectId, Visibility>, TimelineError> {
    let mut seen = BTreeSet::new();
    let mut initial = BTreeMap::new();
    for &i in &hierarchy.traversal {
        cancel(check)?;
        let Entry::Container(container) = Entry::at(timeline, i) else {
            continue;
        };
        match container.presentation {
            None => (),
            Some(PresentationRole::MainSequence) => {
                if container.kind != ContainerKind::Sequence {
                    return Err(invalid(
                        Some(&container.id),
                        "main presentation sequence must be a sequence",
                    ));
                }
            }
            Some(PresentationRole::Effect { preset, .. }) => {
                if container.kind != ContainerKind::Parallel {
                    return Err(invalid(
                        Some(&container.id),
                        "presentation preset requires parallel behaviors",
                    ));
                }
                let children = &hierarchy.children[i];
                let node = |index: usize| -> Result<&TimingNode, TimelineError> {
                    match Entry::at(timeline, children[index]) {
                        Entry::Leaf(node) => Ok(node),
                        _ => Err(invalid(
                            Some(&container.id),
                            "presentation preset requires leaf behaviors",
                        )),
                    }
                };
                let fade = matches!(
                    preset,
                    PresentationPreset::FadeIn | PresentationPreset::FadeOut
                );
                if children.len() != if fade { 2 } else { 1 } {
                    return Err(invalid(
                        Some(&container.id),
                        "presentation preset behavior count",
                    ));
                }
                let first = node(0)?;
                let matches = if fade {
                    let second = node(1)?;
                    let (filter, visibility, direction, value) =
                        if preset == PresentationPreset::FadeIn {
                            (second, first, FadeTransition::In, Visibility::Visible)
                        } else {
                            (first, second, FadeTransition::Out, Visibility::Hidden)
                        };
                    first.target() == second.target()
                        && matches!(filter.effect, Effect::Fade { transition, .. } if transition == direction)
                        && matches!(visibility.effect, Effect::SetVisibility { value: v, .. } if v == value)
                } else {
                    matches!(
                        (preset, &first.effect),
                        (
                            PresentationPreset::Appear,
                            Effect::SetVisibility {
                                value: Visibility::Visible,
                                ..
                            }
                        ) | (
                            PresentationPreset::Disappear,
                            Effect::SetVisibility {
                                value: Visibility::Hidden,
                                ..
                            }
                        ) | (PresentationPreset::Spin, Effect::Rotation { .. })
                            | (PresentationPreset::GrowShrink, Effect::Scale { .. })
                            | (
                                PresentationPreset::CustomMotion,
                                Effect::MotionLine { .. } | Effect::MotionPath { .. }
                            )
                    )
                };
                if !matches {
                    return Err(invalid(
                        Some(&container.id),
                        "presentation preset and behaviors disagree",
                    ));
                }
                // The first editorial effect for an object establishes its
                // presentation baseline. A later entrance cannot hide an object
                // whose earlier effect is an exit or emphasis. Playback reset
                // restores this baseline without changing the document itself.
                if seen.insert(first.target().clone())
                    && matches!(
                        preset,
                        PresentationPreset::Appear | PresentationPreset::FadeIn
                    )
                {
                    initial.insert(first.target().clone(), Visibility::Hidden);
                }
            }
        }
    }
    Ok(initial)
}
