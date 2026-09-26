use super::*;
use crate::{A, P, R};
use mo_xml::{ExpandedName, mce};

pub const PROFILE_ID: &str = "musteroffice.pml-source-mce/1";

pub(super) fn profile() -> mce::Profile {
    mce::Profile {
        understood_namespaces: [P, A, R, "http://www.w3.org/XML/1998/namespace", ""]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        extension_elements: [P, A]
            .into_iter()
            .map(|ns| ExpandedName {
                namespace: ns.into(),
                local: "extLst".into(),
            })
            .collect(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCompatibility {
    pub selections: Vec<SourceCompatibilitySelection>,
    pub ignored_elements: u32,
    pub ignored_attributes: u32,
    pub unwrapped_elements: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCompatibilitySelection {
    pub source_ordinal: u32,
    pub branches: Vec<SourceCompatibilityBranch>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCompatibilityBranch {
    pub source_ordinal: u32,
    pub requires: Vec<String>,
    pub fallback: bool,
    pub selected: bool,
}

pub(super) fn record(summary: mce::Summary) -> Result<SourceCompatibility, crate::PptxError> {
    fn number(value: usize) -> Result<u32, crate::PptxError> {
        value
            .try_into()
            .map_err(|_| crate::PptxError::Limit("compatibility index"))
    }
    Ok(SourceCompatibility {
        ignored_elements: number(summary.ignored_elements)?,
        ignored_attributes: number(summary.ignored_attributes)?,
        unwrapped_elements: number(summary.unwrapped_elements)?,
        selections: summary
            .selections
            .into_iter()
            .map(|s| {
                Ok(SourceCompatibilitySelection {
                    source_ordinal: number(s.source_ordinal)?,
                    branches: s
                        .branches
                        .into_iter()
                        .map(|b| {
                            Ok(SourceCompatibilityBranch {
                                source_ordinal: number(b.source_ordinal)?,
                                requires: b.requires,
                                fallback: b.fallback,
                                selected: b.selected,
                            })
                        })
                        .collect::<Result<_, crate::PptxError>>()?,
                })
            })
            .collect::<Result<_, crate::PptxError>>()?,
    })
}
