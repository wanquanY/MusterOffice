//! Source-bound effect declarations. Flat physical-node references prevent
//! recursive effect/fill copying; evaluation and graph linking are separate.
mod definition;
mod read;
use super::{drawingml::SourceColor, fill::SourceFillReference};
pub use definition::*;
pub(super) use read::{Children, Slots};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceEffectNode {
    pub source_ordinal: u32,
    pub definition: SourceEffectDefinition,
    /// Only this node's uninterpreted properties. Children have their own nodes.
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceEffectProperties {
    pub source_ordinal: u32,
    pub definition: SourceEffectPropertiesDefinition,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourceEffectPropertiesDefinition {
    List { nodes: Vec<u32> },
    Dag { root: u32 },
}
impl SourceEffectProperties {
    /// A syntactically empty list explicitly blocks inherited effects. A DAG
    /// requires its own semantic interpretation, even when it has no children.
    pub fn is_explicitly_empty_list(&self) -> bool {
        self.retained_ordinals.is_empty()
            && matches!(&self.definition, SourceEffectPropertiesDefinition::List { nodes } if nodes.is_empty())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceEffectStyle {
    pub source_ordinal: u32,
    pub effects: SourceEffectProperties,
    /// Includes 3D properties until their independent source family is parsed.
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceEffectReference {
    pub source_ordinal: u32,
    pub index: u32,
    pub color: Option<SourceColor>,
    pub retained_ordinals: Vec<u32>,
}
impl From<SourceFillReference> for SourceEffectReference {
    fn from(v: SourceFillReference) -> Self {
        Self {
            source_ordinal: v.source_ordinal,
            index: v.index,
            color: v.color,
            retained_ordinals: v.retained_ordinals,
        }
    }
}
macro_rules! native_enum { ($name:ident { $($variant:ident => $spelling:literal),+ $(,)? }) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub enum $name { $(#[serde(rename=$spelling)] $variant),+ }
}; }
native_enum!(NativeBlendMode { Over=>"over", Multiply=>"mult", Screen=>"screen", Darken=>"darken", Lighten=>"lighten" });
native_enum!(NativeEffectContainerType { Sibling=>"sib", Tree=>"tree" });
native_enum!(NativePresetShadow {
    Shadow1=>"shdw1",Shadow2=>"shdw2",Shadow3=>"shdw3",Shadow4=>"shdw4",Shadow5=>"shdw5",
    Shadow6=>"shdw6",Shadow7=>"shdw7",Shadow8=>"shdw8",Shadow9=>"shdw9",Shadow10=>"shdw10",
    Shadow11=>"shdw11",Shadow12=>"shdw12",Shadow13=>"shdw13",Shadow14=>"shdw14",Shadow15=>"shdw15",
    Shadow16=>"shdw16",Shadow17=>"shdw17",Shadow18=>"shdw18",Shadow19=>"shdw19",Shadow20=>"shdw20"
});
