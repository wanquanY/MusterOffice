//! Source-bound native text declarations. This flat catalog shares paragraph,
//! character and paint records across bodies, list levels and master defaults.
//! Text leaves remain in the existing edit projection; layout is not inferred.
mod attributes;
pub mod body;
pub mod cascade;
pub mod fonts;
mod grammar;
mod names;
pub mod paint;
mod read;
use super::{drawingml::*, effects::*, fill::*, line::*};
pub use attributes::*;
pub use names::*;
pub(super) use read::{Budget, Reader};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextCatalog {
    /// Physical ordinals in this part, in discovery order. A root is a txBody,
    /// txStyles, defaultTextStyle, fontRef or a theme bodyPr/lstStyle root.
    /// Relationships are resolved later.
    pub roots: Vec<SourceTextRoot>,
    pub nodes: BTreeMap<u32, SourceTextNode>,
    pub effect_nodes: BTreeMap<u32, SourceEffectNode>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextRoot {
    pub source_ordinal: u32,
    /// Physical shape ID in this part; None for presentation/master defaults.
    pub owner: Option<u32>,
}
impl SourceTextCatalog {
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextNode {
    /// Native local name. Reader grammar validates its namespace and context.
    pub element: NativeTextElement,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub value: SourceTextValue,
    /// Uninterpreted attributes/subtrees cannot become resolved text semantics.
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceTextValue {
    Container {},
    Body {
        attributes: Box<SourceTextBodyAttributes>,
    },
    Paragraph {
        attributes: Box<SourceTextParagraphAttributes>,
    },
    Character {
        attributes: Box<SourceTextCharacterAttributes>,
    },
    Font {
        font: Box<SourceTextFont>,
    },
    FontReference {
        index: NativeFontCollectionIndex,
    },
    Autofit {
        font_scale: Option<NativePercentage>,
        line_spacing_reduction: Option<NativePercentage>,
    },
    Percentage {
        value: NativePercentage,
    },
    Points {
        value: i32,
    },
    AutoNumber {
        scheme: NativeTextAutonumber,
        start_at: Option<u16>,
    },
    BulletCharacter {
        character: String,
    },
    Tab {
        position: Option<NativeCoordinate>,
        alignment: Option<NativeTextTabAlign>,
    },
    Field {
        id: String,
        field_type: Option<String>,
    },
    Hyperlink {
        attributes: Box<SourceTextHyperlinkAttributes>,
    },
    RightToLeft {
        value: Option<bool>,
    },
    Fill {
        fill: Box<SourceFill>,
    },
    Line {
        line: Box<SourceLine>,
    },
    Effects {
        effects: SourceEffectProperties,
    },
    Color {
        color: Box<SourceColor>,
    },
}
