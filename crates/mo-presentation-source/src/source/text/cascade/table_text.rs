//! Cell scopes and references to actual table declarations. No synthetic shape,
//! copied text body, or counterfeit SourceTextCatalog node is introduced.
use super::*;
use crate::source::table::{
    SourceCellAddress,
    grid::{NativeTableGrid, NativeTableGridError, NativeTableGridLimits},
    styles::*,
};

pub enum TableTextBinding<'a> {
    Bound { table: TableTextResolver<'a> },
    Unresolved { reason: TextCascadeUnresolved },
}
/// Bind the object, merge topology and table style once, then resolve physical
/// cells independently. Hosts retain the inspected index for this lifetime.
pub struct TableTextResolver<'a> {
    index: &'a SourceIndex,
    context: chain::Context<'a>,
    grid: std::sync::Arc<NativeTableGrid<'a>>,
    style: std::sync::Arc<BoundTableStyle<'a>>,
}
fn selection(error: TableStyleSelectionError) -> Failure {
    if error == TableStyleSelectionError::Cancelled {
        PptxError::Cancelled.into()
    } else {
        TextCascadeUnresolved::TableStyle { reason: error }.into()
    }
}
impl<'a> TableTextResolver<'a> {
    pub fn bind(
        index: &'a SourceIndex,
        expected: &mo_common::Digest,
        object: &SourceObjectRef,
        grid_limits: NativeTableGridLimits,
        limits: TextCascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextBinding<'a>, PptxError> {
        Self::bind_impl(index, expected, object, grid_limits, None, limits, check)
    }
    pub fn bind_grid(
        index: &'a SourceIndex,
        expected: &mo_common::Digest,
        object: &SourceObjectRef,
        grid: std::sync::Arc<NativeTableGrid<'a>>,
        limits: TextCascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextBinding<'a>, PptxError> {
        Self::bind_impl(
            index,
            expected,
            object,
            NativeTableGridLimits::default(),
            Some(grid),
            limits,
            check,
        )
    }
    fn bind_impl(
        index: &'a SourceIndex,
        expected: &mo_common::Digest,
        object: &SourceObjectRef,
        grid_limits: NativeTableGridLimits,
        grid: Option<std::sync::Arc<NativeTableGrid<'a>>>,
        limits: TextCascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextBinding<'a>, PptxError> {
        cancelled(check)?;
        if &index.source_sha256 != expected {
            return Err(conflict());
        }
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
            runs: 0,
        };
        let result = (|| -> Result<Self, Failure> {
            let context = chain::Context::new(index, object, &mut budget)?;
            let native = context.target.object.table.as_ref().ok_or_else(conflict)?;
            let grid = if let Some(grid) = grid {
                if !std::ptr::eq(grid.table(), native) {
                    return Err(conflict().into());
                }
                grid
            } else {
                std::sync::Arc::new(
                    NativeTableGrid::compile(native, grid_limits, check).map_err(|e| match e {
                        NativeTableGridError::Invalid(reason) => {
                            Failure::Unresolved(TextCascadeUnresolved::TableGrid { reason })
                        }
                        NativeTableGridError::Limit => PptxError::Limit("text table grid").into(),
                        NativeTableGridError::Cancelled => PptxError::Cancelled.into(),
                    })?,
                )
            };
            let style = BoundTableStyle::bind(native, index.table_styles.as_deref(), check)
                .map_err(selection)?;
            Ok(Self {
                index,
                context,
                grid,
                style: std::sync::Arc::new(style),
            })
        })();
        match result {
            Ok(table) => Ok(TableTextBinding::Bound { table }),
            Err(Failure::Unresolved(reason)) => Ok(TableTextBinding::Unresolved { reason }),
            Err(Failure::Abort(e)) => Err(e),
        }
    }
    pub fn bind_prepared(
        table: &crate::source::prepared::PreparedSourceTable<'a>,
        limits: TextCascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextBinding<'a>, PptxError> {
        cancelled(check)?;
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
            runs: 0,
        };
        let result = (|| -> Result<Self, Failure> {
            let context = chain::Context::prepared_table(table, &mut budget)?;
            let style = table.style().map_err(selection)?;
            Ok(Self {
                index: table.index(),
                context,
                grid: table.shared_grid(),
                style,
            })
        })();
        match result {
            Ok(table) => Ok(TableTextBinding::Bound { table }),
            Err(Failure::Unresolved(reason)) => Ok(TableTextBinding::Unresolved { reason }),
            Err(Failure::Abort(e)) => Err(e),
        }
    }
    pub fn object(&self) -> &'a SourceObject {
        self.context.target.object
    }
    pub fn grid(&self) -> &NativeTableGrid<'a> {
        &self.grid
    }
    /// Indices inside the result are cell-local; paragraph_start binds them to
    /// the object's original flat paragraph sequence, including covered cells.
    pub fn resolve(
        &self,
        cell: SourceCellAddress,
        limits: TextCascadeLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TextCascadeOutcome, PptxError> {
        cancelled(check)?;
        let mut budget = Budget {
            limits,
            check,
            steps: 0,
            bytes: 0,
            runs: 0,
        };
        let result = (|| -> Result<CascadedText, Failure> {
            let native_cell = self.grid.cell(cell).ok_or_else(conflict)?;
            let root = self
                .context
                .target
                .body(Some(cell))?
                .ok_or(TextCascadeUnresolved::NoTextBody {})?;
            root.checked(&mut budget)?;
            let layers = self
                .style
                .cell(&self.grid, cell, check)
                .map_err(selection)?;
            let selected = layers.text(check).map_err(selection)?;
            let table = layers.definition().map(|(style, location)| CharacterLayer {
                selected,
                source: match location {
                    TableStyleLocation::Inline => TableTextStyleSource::Inline {
                        object: self.context.target.reference(),
                    },
                    TableStyleLocation::Shared(part) => TableTextStyleSource::Catalog {
                        part: part.part.clone(),
                        style_id: style.style_id.to_ascii_uppercase(),
                    },
                },
            });
            let font_reference = table.as_ref().and_then(CharacterLayer::font_reference);
            let scope = Scope {
                cell: Some(cell),
                table,
            };
            let mut paragraphs = vec![];
            let mut levels = BTreeMap::new();
            for p in root.children(&mut budget)? {
                if p.node()?.element != NativeTextElement::P {
                    continue;
                }
                if paragraphs.len() >= limits.max_paragraphs {
                    return Err(PptxError::Limit("text cascade paragraphs").into());
                }
                if paragraphs.len() >= native_cell.paragraph_count as usize {
                    return Err(conflict().into());
                }
                let flat = native_cell.paragraph_start as usize + paragraphs.len();
                paragraphs.push(paragraph(
                    &p,
                    self.object(),
                    flat,
                    &self.context,
                    &scope,
                    &mut levels,
                    &mut budget,
                )?);
            }
            if paragraphs.len() != native_cell.paragraph_count as usize || paragraphs.is_empty() {
                return Err(conflict().into());
            }
            budget.bytes(self.context.target.reference().part.len() + 256)?;
            cancelled(check)?;
            Ok(CascadedText {
                profile: PROFILE.into(),
                source_sha256: self.index.source_sha256.clone(),
                object: self.context.target.reference(),
                cell: Some(cell),
                paragraph_start: native_cell.paragraph_start,
                font_reference,
                paragraphs,
            })
        })();
        match result {
            Ok(text) => Ok(TextCascadeOutcome::Cascaded {
                text: Box::new(text),
            }),
            Err(Failure::Unresolved(reason)) => Ok(TextCascadeOutcome::Unresolved { reason }),
            Err(Failure::Abort(e)) => Err(e),
        }
    }
}

#[derive(Clone)]
pub(super) struct CharacterLayer<'a> {
    selected: SelectedTableTextStyle<'a>,
    pub source: TableTextStyleSource,
}
impl CharacterLayer<'_> {
    fn origin<T>(&self, selected: &SelectedTableStyleValue<T>) -> TextStyleOrigin {
        TextStyleOrigin::TableStyle {
            source: self.source.clone(),
            region: selected.region,
            source_ordinal: selected.source_ordinal,
        }
    }
    fn font_reference(&self) -> Option<TextStyleDeclaration> {
        let font = self.selected.font?;
        matches!(font.value, SourceTableFontStyle::Reference { .. }).then(|| TextStyleDeclaration {
            element: NativeTextElement::FontRef,
            origin: self.origin(&font),
        })
    }
    pub fn apply(
        &self,
        to: &mut CascadedCharacterStyle,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        budget.step()?;
        for (selected, bold) in [(self.selected.bold, true), (self.selected.italic, false)] {
            if let Some(v) = selected {
                let mut attributes = SourceTextCharacterAttributes::default();
                if bold {
                    attributes.bold = Some(v.value);
                } else {
                    attributes.italic = Some(v.value);
                }
                properties::character(
                    &mut to.attributes,
                    &attributes,
                    &mut to.origins,
                    &self.origin(&v),
                    budget,
                )?;
            }
        }
        if let Some(font) = self.selected.font {
            let element = match font.value {
                SourceTableFontStyle::Collection { .. } => NativeTextElement::Font,
                SourceTableFontStyle::Reference { .. } => NativeTextElement::FontRef,
            };
            for slot in [
                CharacterSlot::Latin,
                CharacterSlot::EastAsian,
                CharacterSlot::ComplexScript,
            ] {
                if let std::collections::btree_map::Entry::Vacant(e) = to.declarations.entry(slot) {
                    let origin = self.origin(&font);
                    budget.bytes(origin.bytes() + 32)?;
                    e.insert(TextStyleDeclaration { element, origin });
                }
            }
        }
        if let Some(color) = self.selected.color
            && let std::collections::btree_map::Entry::Vacant(e) =
                to.declarations.entry(CharacterSlot::Fill)
        {
            let origin = self.origin(&color);
            budget.bytes(origin.bytes() + 32)?;
            e.insert(TextStyleDeclaration {
                element: color_element(color.value),
                origin,
            });
        }
        Ok(())
    }
}
/// Return a native element name from the typed color, preserving its own ordinal.
pub fn color_element(color: &SourceColor) -> NativeTextElement {
    match color.value {
        SourceColorValue::Srgb { .. } => NativeTextElement::SrgbClr,
        SourceColorValue::ScRgb { .. } => NativeTextElement::ScrgbClr,
        SourceColorValue::Hsl { .. } => NativeTextElement::HslClr,
        SourceColorValue::System { .. } => NativeTextElement::SysClr,
        SourceColorValue::Scheme { .. } => NativeTextElement::SchemeClr,
        SourceColorValue::Preset { .. } => NativeTextElement::PrstClr,
    }
}
pub enum TableTextDeclaration<'a> {
    Font(&'a SourceTableFontStyle),
    Color(&'a SourceColor),
}
/// Resolve only the actual selected region and ordinal, never another style or
/// catalog node with a coincident ordinal. This takes an inspected source index.
pub fn table_declaration<'a>(
    index: &'a SourceIndex,
    reference: &TextStyleDeclaration,
) -> Result<TableTextDeclaration<'a>, PptxError> {
    let TextStyleOrigin::TableStyle {
        source,
        region,
        source_ordinal,
    } = &reference.origin
    else {
        return Err(conflict());
    };
    let style = match source {
        TableTextStyleSource::Inline { object } => index
            .surfaces
            .get(&object.part)
            .and_then(|s| s.objects.iter().find(|o| o.native_id == object.native_id))
            .and_then(|o| o.table.as_ref())
            .and_then(|t| t.properties.as_ref())
            .filter(|p| p.style_id.is_none())
            .and_then(|p| p.inline_style.as_deref())
            .ok_or_else(conflict)?,
        TableTextStyleSource::Catalog { part, style_id } => {
            let catalog = index
                .table_styles
                .as_deref()
                .filter(|c| &c.part == part && c.retained_ordinals.is_empty())
                .ok_or_else(conflict)?;
            catalog
                .styles
                .get(style_id)
                .filter(|s| s.style_id.eq_ignore_ascii_case(style_id))
                .ok_or_else(conflict)?
        }
    };
    let part = style
        .parts
        .get(region)
        .filter(|p| p.retained_ordinals.is_empty() && style.retained_ordinals.is_empty())
        .ok_or_else(conflict)?;
    let text = part.text.as_ref().ok_or_else(conflict)?;
    if let Some(font) = &text.font {
        let (ordinal, element, color) = match font {
            SourceTableFontStyle::Collection { source_ordinal, .. } => {
                (*source_ordinal, NativeTextElement::Font, None)
            }
            SourceTableFontStyle::Reference {
                source_ordinal,
                color,
                ..
            } => (*source_ordinal, NativeTextElement::FontRef, color.as_ref()),
        };
        if ordinal == *source_ordinal && reference.element == element {
            return Ok(TableTextDeclaration::Font(font));
        }
        if let Some(color) = color
            && color.source_ordinal == *source_ordinal
            && color_element(color) == reference.element
        {
            return Ok(TableTextDeclaration::Color(color));
        }
    }
    if let Some(color) = &text.color
        && color.source_ordinal == *source_ordinal
        && color_element(color) == reference.element
    {
        return Ok(TableTextDeclaration::Color(color));
    }
    Err(conflict())
}
