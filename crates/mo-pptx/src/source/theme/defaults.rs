//! Text declarations within theme object defaults, using the common text reader.
use super::*;
use crate::A;
use mo_xml::{Element, XmlError};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum SourceThemeDefaultKind {
    #[serde(rename = "txDef")]
    Text,
    #[serde(rename = "lnDef")]
    Line,
    #[serde(rename = "spDef")]
    Shape,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceThemeTextDefault {
    pub source_ordinal: u32,
    pub text: text::SourceTextCatalog,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceThemeTextDefaults {
    pub source_ordinal: u32,
    pub entries: BTreeMap<SourceThemeDefaultKind, SourceThemeTextDefault>,
    pub retained_ordinals: Vec<u32>,
}

pub(super) struct Reader {
    pub depth: usize,
    result: SourceThemeTextDefaults,
    current: Option<SourceThemeDefaultKind>,
    text: Option<text::Reader>,
    opaque: Option<usize>,
    rank: u8,
    entry_rank: u8,
    style: Option<Style>,
}
struct Style {
    depth: usize,
    rank: u8,
    font: Option<u32>,
    retained: Vec<u32>,
}
fn unknown_attributes(e: &Element, ordinal: u32, retained: &mut Vec<u32>) {
    if e.attributes
        .iter()
        .any(|a| a.name.namespace != "http://schemas.openxmlformats.org/markup-compatibility/2006")
    {
        retained.push(ordinal);
    }
}
impl Reader {
    pub fn new(e: &Element, depth: usize, ordinal: u32) -> Self {
        let mut retained_ordinals = vec![];
        unknown_attributes(e, ordinal, &mut retained_ordinals);
        Self {
            depth,
            result: SourceThemeTextDefaults {
                source_ordinal: ordinal,
                entries: BTreeMap::new(),
                retained_ordinals,
            },
            current: None,
            text: None,
            opaque: None,
            rank: 0,
            entry_rank: 0,
            style: None,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        e: &Element,
        depth: usize,
        ordinal: u32,
        extension: bool,
        budget: &mut text::Budget,
        line_budget: &mut line::Budget,
        paint_budget: &mut paint::Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        if let Some(reader) = &mut self.text {
            return reader.start(
                e,
                depth,
                ordinal,
                extension,
                budget,
                line_budget,
                paint_budget,
                limits,
            );
        }
        if self.opaque.is_some() {
            return Ok(());
        }
        if let Some(style) = &mut self.style {
            if depth != style.depth + 1 {
                return Err(malformed("theme text style nesting"));
            }
            let rank = if !extension && e.name.namespace == A {
                match e.name.local.as_str() {
                    "lnRef" => 1,
                    "fillRef" => 2,
                    "effectRef" => 3,
                    "fontRef" => 4,
                    _ => 0,
                }
            } else {
                0
            };
            if rank != 0 {
                if rank <= style.rank {
                    return Err(malformed("duplicate or out-of-order theme style reference"));
                }
                style.rank = rank;
                if rank == 4 {
                    style.font = Some(ordinal);
                    self.text = Some(text::Reader::new(e, depth, ordinal, None, budget, limits)?);
                    return Ok(());
                }
            } else {
                style.retained.push(ordinal);
            }
            // Line/fill/effect references remain separate property families.
            // Never interpret nested fontRef inside one of those siblings.
            self.opaque = Some(depth);
            return Ok(());
        }
        if !extension && e.name.namespace == A {
            if depth == self.depth + 1 {
                let kind_rank = match e.name.local.as_str() {
                    "spDef" => Some((SourceThemeDefaultKind::Shape, 1)),
                    "lnDef" => Some((SourceThemeDefaultKind::Line, 2)),
                    "txDef" => Some((SourceThemeDefaultKind::Text, 3)),
                    _ => None,
                };
                if let Some((kind, rank)) = kind_rank {
                    if rank <= self.rank {
                        return Err(malformed("duplicate or out-of-order theme object default"));
                    }
                    self.rank = rank;
                    self.entry_rank = 0;
                    let mut retained_ordinals = vec![];
                    unknown_attributes(e, ordinal, &mut retained_ordinals);
                    self.result.entries.insert(
                        kind,
                        SourceThemeTextDefault {
                            source_ordinal: ordinal,
                            text: text::SourceTextCatalog::default(),
                            retained_ordinals,
                        },
                    );
                    self.current = Some(kind);
                    return Ok(());
                }
            } else if depth == self.depth + 2 && self.current.is_some() {
                let rank = match e.name.local.as_str() {
                    "spPr" => 1,
                    "bodyPr" => 2,
                    "lstStyle" => 3,
                    "style" => 4,
                    "extLst" => 5,
                    _ => 0,
                };
                if rank != 0 {
                    if rank <= self.entry_rank {
                        return Err(malformed("duplicate or out-of-order theme default child"));
                    }
                    self.entry_rank = rank;
                }
                if matches!(e.name.local.as_str(), "bodyPr" | "lstStyle") {
                    self.text = Some(text::Reader::new(e, depth, ordinal, None, budget, limits)?);
                    return Ok(());
                }
                if e.name.local == "style" {
                    let mut retained = vec![];
                    unknown_attributes(e, ordinal, &mut retained);
                    self.style = Some(Style {
                        depth,
                        rank: 0,
                        font: None,
                        retained,
                    });
                    return Ok(());
                }
                // Shape properties are not a font-reference context.
                if e.name.local == "spPr" {
                    self.opaque = Some(depth);
                    return Ok(());
                }
            }
        }
        if let Some(kind) = self.current {
            self.result
                .entries
                .get_mut(&kind)
                .expect("default entry")
                .retained_ordinals
                .push(ordinal);
        } else {
            self.result.retained_ordinals.push(ordinal);
        }
        self.opaque = Some(depth);
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if let Some(reader) = &mut self.text {
            reader.end(depth)?;
            if reader.depth == depth {
                self.text.take().expect("text reader").finish(
                    &mut self
                        .result
                        .entries
                        .get_mut(&self.current.expect("entry"))
                        .expect("default entry")
                        .text,
                )?;
            }
        } else if self.opaque == Some(depth) {
            self.opaque = None;
        } else if self.style.as_ref().is_some_and(|s| s.depth == depth) {
            let style = self.style.take().expect("style scope");
            let font = style
                .font
                .ok_or_else(|| malformed("missing theme style font reference"))?;
            let catalog = &mut self
                .result
                .entries
                .get_mut(&self.current.expect("default entry"))
                .expect("entry")
                .text;
            catalog
                .nodes
                .get_mut(&font)
                .ok_or_else(|| malformed("theme font reference binding"))?
                .retained_ordinals
                .extend(style.retained);
        } else if self.opaque.is_none() && depth == self.depth + 1 {
            self.current = None;
        }
        Ok(())
    }
    pub fn characters(&self, value: &str) -> Result<(), XmlError> {
        if let Some(reader) = &self.text {
            reader.text(value)
        } else if self.opaque.is_none() && !value.trim().is_empty() {
            Err(malformed("unexpected theme default text"))
        } else {
            Ok(())
        }
    }
    pub fn finish(self) -> Result<SourceThemeTextDefaults, XmlError> {
        if self.text.is_some()
            || self.current.is_some()
            || self.opaque.is_some()
            || self.style.is_some()
        {
            return Err(malformed("unclosed theme defaults"));
        }
        Ok(self.result)
    }
}
