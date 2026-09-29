//! Coverage of presentation structure outside delegated semantic readers.
//! Unknown appearance is source data, never permission to omit visible content.
//! Locking, hyperlink actions and timing do not alter the static authoring view;
//! this audit does not claim interaction, playback or editing support for them.
use super::{SourceSurface, SurfaceKind};
use crate::{A, P};
use mo_xml::{Element, ExpandedName, XmlError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceVisualIssueKind {
    Element,
    Attribute,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceVisualIssue {
    pub source_ordinal: u32,
    pub kind: SourceVisualIssueKind,
    pub namespace: String,
    pub local_name: String,
}
#[derive(Default)]
pub(super) struct Reader {
    opaque_depth: Option<usize>,
}
impl Reader {
    pub fn text(&self, text: &str, delegated: bool) -> Result<(), XmlError> {
        if !delegated && self.opaque_depth.is_none() && !text.trim().is_empty() {
            return Err(super::malformed(
                "text outside native text/semantic content",
            ));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) {
        if self.opaque_depth == Some(depth) {
            self.opaque_depth = None;
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        e: &Element,
        stack: &[ExpandedName],
        ordinal: u32,
        owner: Option<(usize, usize)>,
        delegated: bool,
        extension: bool,
        surface: &mut SourceSurface,
    ) -> Result<(), XmlError> {
        let depth = stack.len();
        if self.opaque_depth.is_some() || delegated {
            return Ok(());
        }
        let issues = if let Some((index, _)) = owner {
            &mut surface.objects[index].visual_issues
        } else {
            &mut surface.visual_issues
        };
        let mut issue = |kind, name: &ExpandedName| {
            issues.push(SourceVisualIssue {
                source_ordinal: ordinal,
                kind,
                namespace: name.namespace.clone(),
                local_name: name.local.clone(),
            })
        };
        let parent = stack.last();
        let parent_is = |ns, local| parent.is_some_and(|n| n.is(ns, local));
        let name = e.name.local.as_str();
        // A semantic delegate owns its entire subtree and retained attributes.
        // The remaining structural vocabulary is exact and context-sensitive.
        let mut skip = false;
        let allowed: Option<&[&str]> = if extension {
            None
        } else if depth == 0 {
            Some(match surface.kind {
                SurfaceKind::Slide => &["show", "showMasterSp", "showMasterPhAnim"],
                SurfaceKind::Layout => &[
                    "showMasterSp",
                    "showMasterPhAnim",
                    "matchingName",
                    "type",
                    "preserve",
                    "userDrawn",
                ],
                SurfaceKind::Master => &["preserve"],
            })
        } else if depth == 1 && e.name.namespace == P {
            match name {
                "cSld" => Some(&["name"]),
                "clrMap" => Some(&[
                    "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4",
                    "accent5", "accent6", "hlink", "folHlink",
                ]),
                "clrMapOvr" => Some(&[]),
                "hf" if matches!(surface.kind, SurfaceKind::Master | SurfaceKind::Layout) => {
                    Some(&["dt", "ftr", "hdr", "sldNum"])
                }
                "timing" | "transition" | "sldLayoutIdLst" | "txStyles" => {
                    skip = true;
                    Some(&[])
                }
                _ => None,
            }
        } else if depth == 2 && parent_is(P, "clrMapOvr") && e.name.is(A, "overrideClrMapping") {
            Some(&[
                "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4", "accent5",
                "accent6", "hlink", "folHlink",
            ])
        } else if depth == 2
            && ((parent_is(P, "clrMapOvr") && e.name.is(A, "masterClrMapping"))
                || (parent_is(P, "cSld") && e.name.is(P, "spTree")))
        {
            Some(&[])
        } else if owner.is_some_and(|(_, d)| d == depth) {
            Some(if e.name.is(P, "sp") {
                &["useBgFill"]
            } else {
                &[]
            })
        } else if (parent_is(P, "spTree") || parent_is(P, "grpSp"))
            && e.name.namespace == P
            && matches!(name, "nvGrpSpPr" | "grpSpPr")
        {
            Some(&[])
        } else if owner.is_some_and(|(_, d)| depth == d + 1) && e.name.namespace == P {
            match name {
                "nvSpPr" | "nvPicPr" | "nvCxnSpPr" | "nvGraphicFramePr" | "spPr" | "style" => {
                    Some(&[])
                }
                "txBody" => {
                    skip = true;
                    Some(&[])
                }
                _ => None,
            }
        } else if e.name.namespace == P
            && parent.is_some_and(|n| {
                n.namespace == P
                    && matches!(
                        n.local.as_str(),
                        "nvSpPr" | "nvPicPr" | "nvCxnSpPr" | "nvGrpSpPr" | "nvGraphicFramePr"
                    )
            })
        {
            match name {
                "cNvPr" => Some(if owner.is_some() {
                    &["id", "name", "descr", "title", "hidden"]
                } else {
                    &["id", "name", "descr", "title"]
                }),
                "cNvSpPr" => Some(&["txBox"]),
                "cNvPicPr" => Some(&["preferRelativeResize"]),
                "cNvCxnSpPr" | "cNvGrpSpPr" | "cNvGraphicFramePr" | "nvPr" => Some(&[]),
                _ => None,
            }
        } else if parent_is(P, "nvPr") && e.name.is(P, "ph") {
            Some(&["type", "orient", "sz", "idx", "hasCustomPrompt"])
        } else if parent_is(P, "graphicFrame")
            && e.name.is(A, "graphic")
            && owner.is_some_and(|(_, d)| depth == d + 1)
        {
            Some(&[])
        } else if parent_is(A, "graphic")
            && e.name.is(A, "graphicData")
            && owner.is_some_and(|(_, d)| depth == d + 2)
            && e.attribute("uri") == Some(super::table::TABLE_URI)
        {
            Some(&["uri"])
        } else if e.name.namespace == A
            && ((parent_is(P, "cNvPr") && matches!(name, "hlinkClick" | "hlinkHover"))
                || ([
                    "cNvSpPr",
                    "cNvPicPr",
                    "cNvCxnSpPr",
                    "cNvGrpSpPr",
                    "cNvGraphicFramePr",
                ]
                .iter()
                .any(|n| parent_is(P, n))
                    && matches!(
                        name,
                        "spLocks"
                            | "picLocks"
                            | "cxnSpLocks"
                            | "grpSpLocks"
                            | "graphicFrameLocks"
                            | "stCxn"
                            | "endCxn"
                    ))
                || (parent_is(P, "style") && name == "fontRef"))
        {
            skip = true;
            Some(&[])
        } else {
            None
        };
        if let Some(allowed) = allowed {
            if !skip {
                for a in &e.attributes {
                    // MCE has already processed control attributes before this
                    // projected event. Namespace declarations aren't attributes.
                    if a.name.namespace
                        == "http://schemas.openxmlformats.org/markup-compatibility/2006"
                    {
                        continue;
                    }
                    if !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()) {
                        issue(SourceVisualIssueKind::Attribute, &a.name);
                    }
                }
            }
        } else {
            issue(SourceVisualIssueKind::Element, &e.name);
            skip = true;
        }
        if skip {
            self.opaque_depth = Some(depth);
        }
        Ok(())
    }
}
