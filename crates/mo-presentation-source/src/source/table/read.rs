//! Streaming table grammar. Subreaders share the part's font/paint/text budgets;
//! unknown declarations keep physical ordinals and cannot become resolved paint.
use super::*;
use crate::{
    A,
    source::{
        self, SourceLimits, boolean, drawingml::enumeration, integer, line, malformed, paint, text,
    },
};
use mo_xml::{Element, XmlError};
use std::collections::BTreeMap;

#[derive(Default)]
pub(in crate::source) struct Budget {
    elements: usize,
    attributes: usize,
    cells: usize,
    styles: usize,
}
impl Budget {
    pub(in crate::source) fn element(
        &mut self,
        e: &Element,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        self.elements = self
            .elements
            .checked_add(1)
            .ok_or(XmlError::Limit("table elements"))?;
        for a in &e.attributes {
            self.attributes = self
                .attributes
                .checked_add(a.value.len())
                .ok_or(XmlError::Limit("table attribute bytes"))?;
        }
        if self.elements > limits.max_table_elements
            || self.attributes > limits.max_table_attribute_bytes
            || self.cells > limits.max_table_cells
        {
            return Err(XmlError::Limit("table declarations"));
        }
        Ok(())
    }
    pub(in crate::source) fn text(
        &mut self,
        text: &str,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        self.attributes = self
            .attributes
            .checked_add(text.len())
            .filter(|n| *n <= limits.max_table_attribute_bytes)
            .ok_or(XmlError::Limit("table attribute bytes"))?;
        Ok(())
    }
    pub(in crate::source) fn style(&mut self, limits: SourceLimits) -> Result<(), XmlError> {
        self.styles = self
            .styles
            .checked_add(1)
            .filter(|n| *n <= limits.max_table_styles)
            .ok_or(XmlError::Limit("table styles"))?;
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Table,
    Properties,
    Grid,
    Column,
    Row,
    Cell,
    CellProperties,
    StyleId,
}
struct Frame {
    kind: Kind,
    rank: u8,
}
#[derive(Clone, Copy)]
enum PaintSlot {
    TableFill,
    TableEffects,
    CellFill,
}
enum Capture {
    Style(Box<styles::Reader>),
    Text {
        style: Box<text::Reader>,
        content: text::ContentReader,
    },
    Line {
        index: usize,
        reader: Box<line::Reader>,
    },
    Paint {
        slot: PaintSlot,
        reader: Box<paint::Reader>,
    },
}
impl Capture {
    fn depth(&self) -> usize {
        match self {
            Self::Style(reader) => reader.depth,
            Self::Text { style, .. } => style.depth,
            Self::Line { reader, .. } => reader.depth,
            Self::Paint { reader, .. } => reader.depth,
        }
    }
}
pub(in crate::source) struct ReadTable {
    pub table: SourceTable,
    pub text: text::SourceTextCatalog,
    pub content: text::ContentResult,
    pub effects: BTreeMap<u32, source::effects::SourceEffectNode>,
}
pub(in crate::source) struct Reader {
    pub depth: usize,
    owner: u32,
    value: SourceTable,
    frames: Vec<Frame>,
    opaque: Option<usize>,
    capture: Option<Capture>,
    catalog: text::SourceTextCatalog,
    roots: text::CatalogRoots,
    content: text::ContentResult,
    effects: BTreeMap<u32, source::effects::SourceEffectNode>,
}
impl Reader {
    pub fn new(
        e: &Element,
        depth: usize,
        ordinal: u32,
        owner: u32,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<Self, XmlError> {
        budget.element(e, limits)?;
        let mut out = Self {
            depth,
            owner,
            value: SourceTable {
                source_ordinal: ordinal,
                properties: None,
                grid_ordinal: 0,
                columns: vec![],
                rows: vec![],
                retained_ordinals: vec![],
            },
            frames: vec![Frame {
                kind: Kind::Table,
                rank: 0,
            }],
            opaque: None,
            capture: None,
            catalog: Default::default(),
            roots: Default::default(),
            content: text::ContentResult {
                paragraphs: vec![],
                bindings: vec![],
            },
            effects: BTreeMap::new(),
        };
        out.attributes(e, ordinal, &[]);
        Ok(out)
    }
    fn cell(&mut self) -> &mut SourceTableCell {
        self.value
            .rows
            .last_mut()
            .expect("open row")
            .cells
            .last_mut()
            .expect("open cell")
    }
    fn retain(&mut self, ordinal: u32) {
        if self.value.retained_ordinals.last() != Some(&ordinal) {
            self.value.retained_ordinals.push(ordinal);
        }
    }
    fn attributes(&mut self, e: &Element, ordinal: u32, allowed: &[&str]) {
        if e.attributes
            .iter()
            .any(|a| !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
        {
            self.retain(ordinal);
        }
    }
    fn order(&mut self, rank: u8, repeated: bool) -> Result<(), XmlError> {
        let frame = self.frames.last_mut().expect("table frame");
        if rank < frame.rank || (rank == frame.rank && !repeated) {
            return Err(malformed("duplicate or out-of-order table declaration"));
        }
        frame.rank = rank;
        Ok(())
    }
    pub fn physical_element(&mut self) {
        if let Some(Capture::Text { content, .. }) = &mut self.capture {
            content.physical_element();
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        e: &Element,
        depth: usize,
        ordinal: u32,
        extension: bool,
        in_alternate: bool,
        budget: &mut Budget,
        text_budget: &mut text::Budget,
        line_budget: &mut line::Budget,
        paint_budget: &mut paint::Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        if let Some(Capture::Style(reader)) = &mut self.capture {
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
        budget.element(e, limits)?;
        if let Some(capture) = &mut self.capture {
            match capture {
                Capture::Style(_) => unreachable!("style delegated before budget charge"),
                Capture::Text { style, content } => {
                    style.start(
                        e,
                        depth,
                        ordinal,
                        extension,
                        text_budget,
                        line_budget,
                        paint_budget,
                        limits,
                    )?;
                    content.start(e, depth, ordinal as usize, in_alternate)?;
                }
                Capture::Line { reader, .. } => reader.start(
                    e,
                    depth,
                    ordinal,
                    extension,
                    line_budget,
                    paint_budget,
                    limits,
                )?,
                Capture::Paint { reader, .. } => {
                    reader.start(e, depth, ordinal, extension, paint_budget, limits)?
                }
            }
            return Ok(());
        }
        if self.opaque.is_some() {
            return Ok(());
        }
        if depth != self.depth + self.frames.len() {
            return Err(malformed("table declaration nesting"));
        }
        if extension || e.name.namespace != A {
            self.retain(ordinal);
            self.opaque = Some(depth);
            return Ok(());
        }
        let name = e.name.local.as_str();
        let parent = self.frames.last().expect("table root").kind;
        let child = match (parent, name) {
            (Kind::Table, "tblPr") => Some((Kind::Properties, 1, false)),
            (Kind::Table, "tblGrid") => Some((Kind::Grid, 2, false)),
            (Kind::Table, "tr") => Some((Kind::Row, 3, true)),
            (Kind::Grid, "gridCol") => Some((Kind::Column, 1, true)),
            (Kind::Row, "tc") => Some((Kind::Cell, 1, true)),
            (Kind::Cell, "tcPr") => Some((Kind::CellProperties, 2, false)),
            (Kind::Properties, "tableStyleId") => Some((Kind::StyleId, 3, false)),
            _ => None,
        };
        if let Some((kind, rank, repeated)) = child {
            self.order(rank, repeated)?;
            match kind {
                Kind::Properties => {
                    self.value.properties = Some(SourceTableProperties {
                        source_ordinal: ordinal,
                        right_to_left: e.attribute("rtl").map(boolean).transpose()?,
                        first_row: e.attribute("firstRow").map(boolean).transpose()?,
                        first_column: e.attribute("firstCol").map(boolean).transpose()?,
                        last_row: e.attribute("lastRow").map(boolean).transpose()?,
                        last_column: e.attribute("lastCol").map(boolean).transpose()?,
                        band_rows: e.attribute("bandRow").map(boolean).transpose()?,
                        band_columns: e.attribute("bandCol").map(boolean).transpose()?,
                        ..Default::default()
                    });
                    self.attributes(
                        e,
                        ordinal,
                        &[
                            "rtl", "firstRow", "firstCol", "lastRow", "lastCol", "bandRow",
                            "bandCol",
                        ],
                    );
                }
                Kind::Grid => {
                    self.value.grid_ordinal = ordinal;
                    self.attributes(e, ordinal, &[]);
                }
                Kind::Column => {
                    self.value.columns.push(SourceTableColumn {
                        source_ordinal: ordinal,
                        width: native_coordinate(e.attribute("w"))?,
                    });
                    self.attributes(e, ordinal, &["w"]);
                }
                Kind::Row => {
                    if self.value.grid_ordinal == 0 {
                        return Err(malformed("table grid must precede rows"));
                    }
                    self.value.rows.push(SourceTableRow {
                        source_ordinal: ordinal,
                        height: native_coordinate(e.attribute("h"))?,
                        cells: vec![],
                    });
                    self.attributes(e, ordinal, &["h"]);
                }
                Kind::Cell => {
                    budget.cells = budget
                        .cells
                        .checked_add(1)
                        .filter(|n| *n <= limits.max_table_cells)
                        .ok_or(XmlError::Limit("table cells"))?;
                    self.value
                        .rows
                        .last_mut()
                        .expect("row")
                        .cells
                        .push(SourceTableCell {
                            source_ordinal: ordinal,
                            native_id: e.attribute("id").map(str::to_owned),
                            row_span: e
                                .attribute("rowSpan")
                                .map(|v| integer(Some(v), "row span"))
                                .transpose()?,
                            grid_span: e
                                .attribute("gridSpan")
                                .map(|v| integer(Some(v), "grid span"))
                                .transpose()?,
                            horizontal_merge: e.attribute("hMerge").map(boolean).transpose()?,
                            vertical_merge: e.attribute("vMerge").map(boolean).transpose()?,
                            paragraph_start: self
                                .content
                                .paragraphs
                                .len()
                                .try_into()
                                .map_err(|_| XmlError::Limit("table paragraphs"))?,
                            ..Default::default()
                        });
                    self.attributes(
                        e,
                        ordinal,
                        &["id", "rowSpan", "gridSpan", "hMerge", "vMerge"],
                    );
                }
                Kind::CellProperties => {
                    let margin = |name| e.attribute(name).map(text::coordinate32).transpose();
                    self.cell().properties = Some(SourceTableCellProperties {
                        source_ordinal: ordinal,
                        margins: SourceTableMargins {
                            left: margin("marL")?,
                            right: margin("marR")?,
                            top: margin("marT")?,
                            bottom: margin("marB")?,
                        },
                        vertical: e.attribute("vert").map(enumeration).transpose()?,
                        horizontal_overflow: e
                            .attribute("horzOverflow")
                            .map(enumeration)
                            .transpose()?,
                        vertical_alignment: e.attribute("anchor").map(enumeration).transpose()?,
                        center_anchor: e.attribute("anchorCtr").map(boolean).transpose()?,
                        ..Default::default()
                    });
                    self.attributes(
                        e,
                        ordinal,
                        &[
                            "marL",
                            "marR",
                            "marT",
                            "marB",
                            "vert",
                            "horzOverflow",
                            "anchor",
                            "anchorCtr",
                        ],
                    );
                }
                Kind::StyleId => {
                    self.value
                        .properties
                        .as_mut()
                        .expect("table properties")
                        .style_id = Some(String::new());
                    self.attributes(e, ordinal, &[]);
                }
                _ => unreachable!("child grammar"),
            }
            self.frames.push(Frame { kind, rank: 0 });
            return Ok(());
        }
        if parent == Kind::Cell && name == "txBody" {
            self.order(1, false)?;
            self.cell().text_body_ordinal = Some(ordinal);
            let cell = SourceCellAddress {
                row: (self.value.rows.len() - 1)
                    .try_into()
                    .map_err(|_| XmlError::Limit("table rows"))?,
                column: (self.value.rows.last().expect("row").cells.len() - 1)
                    .try_into()
                    .map_err(|_| XmlError::Limit("table columns"))?,
            };
            self.capture = Some(Capture::Text {
                style: Box::new(text::Reader::cell_body(
                    e,
                    depth,
                    ordinal,
                    self.owner,
                    cell,
                    text_budget,
                    limits,
                )?),
                content: text::ContentReader::new(depth),
            });
            return Ok(());
        }
        if parent == Kind::Properties && name == "tableStyle" {
            self.order(3, false)?;
            self.capture = Some(Capture::Style(Box::new(styles::Reader::new(
                e, depth, ordinal, budget, limits,
            )?)));
            return Ok(());
        }
        if parent == Kind::CellProperties
            && let Some(index) = ["lnL", "lnR", "lnT", "lnB", "lnTlToBr", "lnBlToTr"]
                .iter()
                .position(|n| *n == name)
        {
            self.order(index as u8 + 1, false)?;
            self.capture = Some(Capture::Line {
                index,
                reader: Box::new(line::Reader::new(e, depth, ordinal, line_budget, limits)?),
            });
            return Ok(());
        }
        let paint = if source::fill::is_fill(&e.name) {
            match parent {
                Kind::Properties => Some((1, PaintSlot::TableFill)),
                Kind::CellProperties => Some((8, PaintSlot::CellFill)),
                _ => None,
            }
        } else if parent == Kind::Properties && paint::is_effect_properties(&e.name) {
            Some((2, PaintSlot::TableEffects))
        } else {
            None
        };
        if let Some((rank, slot)) = paint {
            self.order(rank, false)?;
            self.capture = Some(Capture::Paint {
                slot,
                reader: Box::new(paint::Reader::new(e, depth, ordinal, paint_budget, limits)?),
            });
            return Ok(());
        }
        // Recognized-but-unimplemented native families still participate in
        // sequence/duplicate validation and remain explicit unresolved data.
        match (parent, name) {
            (Kind::CellProperties, "cell3D") => self.order(7, false)?,
            (Kind::CellProperties, "headers") => self.order(9, false)?,
            (_, "extLst") => self.order(255, false)?,
            _ => (),
        }
        self.retain(ordinal);
        self.opaque = Some(depth);
        Ok(())
    }
    pub fn text(
        &mut self,
        value: &str,
        bytes: &mut usize,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        if let Some(capture) = &mut self.capture {
            return match capture {
                Capture::Style(reader) => reader.text(value),
                Capture::Text { style, content } => {
                    style.text(value)?;
                    content.text(value, bytes, limits)
                }
                Capture::Line { reader, .. } => reader.text(value),
                Capture::Paint { reader, .. } => reader.text(value),
            };
        }
        if self.opaque.is_some() {
            return Ok(());
        }
        if self.frames.last().is_some_and(|f| f.kind == Kind::StyleId) {
            budget.text(value, limits)?;
            self.value
                .properties
                .as_mut()
                .expect("table properties")
                .style_id
                .as_mut()
                .expect("style text")
                .push_str(value);
        } else if !value.trim().is_empty() {
            return Err(malformed("text outside table semantic content"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if let Some(capture) = &mut self.capture {
            match capture {
                Capture::Style(reader) => reader.end(depth)?,
                Capture::Text { style, content } => {
                    style.end(depth)?;
                    content.end(depth);
                }
                Capture::Line { reader, .. } => reader.end(depth)?,
                Capture::Paint { reader, .. } => {
                    if depth != reader.depth {
                        reader.end(depth)?;
                    }
                }
            }
            if self.capture.as_ref().expect("capture").depth() != depth {
                return Ok(());
            }
            match self.capture.take().expect("capture") {
                Capture::Style(reader) => {
                    self.value
                        .properties
                        .as_mut()
                        .expect("table properties")
                        .inline_style = Some(Box::new(reader.finish()?));
                }
                Capture::Text { style, content } => {
                    style.finish_indexed(&mut self.catalog, &mut self.roots)?;
                    let content = content.finish()?;
                    let start = self.content.paragraphs.len();
                    self.cell().paragraph_start = start
                        .try_into()
                        .map_err(|_| XmlError::Limit("table paragraphs"))?;
                    self.cell().paragraph_count = content
                        .paragraphs
                        .len()
                        .try_into()
                        .map_err(|_| XmlError::Limit("table paragraphs"))?;
                    self.content.paragraphs.extend(content.paragraphs);
                    self.content.bindings.extend(
                        content
                            .bindings
                            .into_iter()
                            .map(|(p, r, o)| (start + p, r, o)),
                    );
                }
                Capture::Line { index, reader } => {
                    let line::Declaration::Line(line) = reader.finish() else {
                        return Err(malformed("table border reference"));
                    };
                    self.cell()
                        .properties
                        .as_mut()
                        .expect("cell properties")
                        .borders[index] = Some(line);
                }
                Capture::Paint { slot, reader } => {
                    match (slot, reader.finish()?.publish(&mut self.effects)?) {
                        (PaintSlot::TableFill, paint::Declaration::Fill(v)) => {
                            self.value
                                .properties
                                .as_mut()
                                .expect("table properties")
                                .fill = Some(v)
                        }
                        (PaintSlot::TableEffects, paint::Declaration::Effects(v)) => {
                            self.value
                                .properties
                                .as_mut()
                                .expect("table properties")
                                .effects = Some(v)
                        }
                        (PaintSlot::CellFill, paint::Declaration::Fill(v)) => {
                            self.cell()
                                .properties
                                .as_mut()
                                .expect("cell properties")
                                .fill = Some(v)
                        }
                        _ => return Err(malformed("table paint declaration")),
                    }
                }
            }
            return Ok(());
        }
        if let Some(opaque) = self.opaque {
            if opaque == depth {
                self.opaque = None;
            }
            return Ok(());
        }
        if depth != self.depth + self.frames.len() - 1 {
            return Err(malformed("table closing depth"));
        }
        let frame = self.frames.pop().expect("table frame");
        if frame.kind == Kind::Table && self.value.grid_ordinal == 0 {
            return Err(malformed("missing native table grid"));
        }
        Ok(())
    }
    pub fn finish(self) -> Result<ReadTable, XmlError> {
        if !self.frames.is_empty() || self.opaque.is_some() || self.capture.is_some() {
            return Err(malformed("unclosed table"));
        }
        Ok(ReadTable {
            table: self.value,
            text: self.catalog,
            content: self.content,
            effects: self.effects,
        })
    }
}

fn native_coordinate(value: Option<&str>) -> Result<NativeCoordinate, XmlError> {
    NativeCoordinate::try_from(
        value
            .ok_or_else(|| malformed("missing native table coordinate"))?
            .trim()
            .to_owned(),
    )
    .map_err(malformed)
}
