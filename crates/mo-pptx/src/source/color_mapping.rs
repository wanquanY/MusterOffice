//! Author color-map declarations. The map selects theme slots, not RGB values.
use super::{SurfaceKind, malformed, theme::ColorSlot};
use crate::{A, P};
use mo_xml::{Element, ExpandedName, XmlError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceColorMap {
    pub bg1: ColorSlot,
    pub tx1: ColorSlot,
    pub bg2: ColorSlot,
    pub tx2: ColorSlot,
    pub accent1: ColorSlot,
    pub accent2: ColorSlot,
    pub accent3: ColorSlot,
    pub accent4: ColorSlot,
    pub accent5: ColorSlot,
    pub accent6: ColorSlot,
    pub hlink: ColorSlot,
    pub fol_hlink: ColorSlot,
}
impl SourceColorMap {
    fn read(element: &Element) -> Result<Self, XmlError> {
        let slot = |name| {
            let value = element
                .attribute(name)
                .ok_or_else(|| malformed(format!("missing color-map attribute {name}")))?;
            ColorSlot::deserialize(
                serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value.trim()),
            )
            .map_err(|_| malformed("invalid color-map theme slot"))
        };
        Ok(Self {
            bg1: slot("bg1")?,
            tx1: slot("tx1")?,
            bg2: slot("bg2")?,
            tx2: slot("tx2")?,
            accent1: slot("accent1")?,
            accent2: slot("accent2")?,
            accent3: slot("accent3")?,
            accent4: slot("accent4")?,
            accent5: slot("accent5")?,
            accent6: slot("accent6")?,
            hlink: slot("hlink")?,
            fol_hlink: slot("folHlink")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceColorMapping {
    /// a:masterClrMapping, kept distinct from an omitted clrMapOvr.
    Master { source_ordinal: u32 },
    /// p:clrMap on a master or a:overrideClrMapping on a layout/slide.
    Explicit {
        source_ordinal: u32,
        mapping: SourceColorMap,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceColorMapRef {
    pub part: String,
    pub source_ordinal: u32,
}

#[derive(Default)]
pub(super) struct Reader {
    declaration: Option<SourceColorMapping>,
    override_seen: bool,
}
impl Reader {
    pub(super) fn start(
        &mut self,
        element: &Element,
        stack: &[ExpandedName],
        kind: SurfaceKind,
        ordinal: usize,
        extension: bool,
    ) -> Result<(), XmlError> {
        if extension {
            return Ok(());
        }
        let ordinal = || {
            ordinal
                .try_into()
                .map_err(|_| XmlError::Limit("color-map source ordinal"))
        };
        if stack.len() == 1 && element.name.is(P, "clrMap") {
            if kind != SurfaceKind::Master || self.declaration.is_some() {
                return Err(malformed("invalid or duplicate master color map"));
            }
            self.declaration = Some(SourceColorMapping::Explicit {
                source_ordinal: ordinal()?,
                mapping: SourceColorMap::read(element)?,
            });
        } else if stack.len() == 1 && element.name.is(P, "clrMapOvr") {
            if kind == SurfaceKind::Master || self.override_seen {
                return Err(malformed("invalid or duplicate color-map override"));
            }
            self.override_seen = true;
        } else if stack.len() == 2 && stack[1].is(P, "clrMapOvr") {
            if self.declaration.is_some() {
                return Err(malformed("multiple color-map override choices"));
            }
            self.declaration = Some(if element.name.is(A, "masterClrMapping") {
                SourceColorMapping::Master {
                    source_ordinal: ordinal()?,
                }
            } else if element.name.is(A, "overrideClrMapping") {
                SourceColorMapping::Explicit {
                    source_ordinal: ordinal()?,
                    mapping: SourceColorMap::read(element)?,
                }
            } else {
                return Err(malformed("unknown color-map override choice"));
            });
        } else if Self::inside(stack) {
            // Valid extLst subtrees are already opaque in the shared MCE profile.
            return Err(malformed("unexpected color-map child"));
        }
        Ok(())
    }
    pub(super) fn text(
        &self,
        text: &str,
        stack: &[ExpandedName],
        extension: bool,
    ) -> Result<(), XmlError> {
        if !extension && Self::inside(stack) && !text.trim().is_empty() {
            return Err(malformed("unexpected color-map character data"));
        }
        Ok(())
    }
    fn inside(stack: &[ExpandedName]) -> bool {
        stack
            .get(1)
            .is_some_and(|n| n.is(P, "clrMap") || n.is(P, "clrMapOvr"))
    }
    pub(super) fn finish(self) -> Result<Option<SourceColorMapping>, XmlError> {
        if self.override_seen && self.declaration.is_none() {
            return Err(malformed("empty color-map override"));
        }
        Ok(self.declaration)
    }
}
