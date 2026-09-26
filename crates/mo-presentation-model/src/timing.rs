use crate::{ContainerId, Document, ValidationCode, ValidationIssue};
use mo_common::SlideId;
use mo_timeline::{TimelineLimits, TimelinePlan};
use std::collections::BTreeSet;

pub(crate) fn validate(
    document: &Document,
    max_nodes: usize,
    max_depth: usize,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut count = 0usize;
    for (slide, timeline) in &document.timelines {
        let path = format!("/timelines/{slide}");
        count = count.saturating_add(timeline.node_count());
        if count > max_nodes {
            issues.push(ValidationIssue {
                code: ValidationCode::LimitExceeded,
                path,
                message: "document timing node limit".into(),
            });
            break;
        }
        if !document.slides.contains_key(slide) {
            issues.push(ValidationIssue {
                code: ValidationCode::MissingReference,
                path,
                message: "timeline slide does not exist".into(),
            });
            continue;
        }
        match TimelinePlan::compile(
            timeline,
            TimelineLimits {
                max_nodes,
                ..TimelineLimits::default()
            },
            &|| false,
        ) {
            Err(e) => issues.push(ValidationIssue {
                code: ValidationCode::InvalidValue,
                path,
                message: e.to_string(),
            }),
            Ok(plan) => {
                for target in plan.targets() {
                    let mut seen = BTreeSet::new();
                    let mut current = Some(target);
                    let mut owner: Option<&SlideId> = None;
                    while let Some(id) = current {
                        if seen.len() > max_depth || !seen.insert(id) {
                            break;
                        }
                        let Some(object) = document.objects.get(id) else {
                            break;
                        };
                        match &object.parent {
                            ContainerId::Slide(id) => {
                                owner = Some(id);
                                break;
                            }
                            ContainerId::Group(id) => current = Some(id),
                            _ => break,
                        }
                    }
                    if owner != Some(slide) {
                        issues.push(ValidationIssue{code:ValidationCode::OwnershipConflict,path:format!("{path}/targets/{target}"),message:"animation target must belong to this slide, including nested groups".into()});
                    }
                }
            }
        }
    }
    issues
}
