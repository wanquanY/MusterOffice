//! Shared explicit collection selection for theme fonts and native table fonts.
use super::*;
use crate::source::table::styles::SourceTableFontStyle;

pub(super) enum Input {
    Font {
        font: SourceTextFont,
        table_font: Option<TableFontBinding>,
    },
    Reference(NativeFontCollectionIndex),
}
pub(super) fn declaration(
    index: &SourceIndex,
    reference: &TextStyleDeclaration,
    slot: NativeFontSlot,
    script: Option<&str>,
    limits: TypefaceLimits,
    check: &dyn Fn() -> bool,
) -> Result<Result<Input, TypefaceUnresolved>, PptxError> {
    if matches!(reference.origin, TextStyleOrigin::TableStyle { .. }) {
        let TableTextDeclaration::Font(font) = table_declaration(index, reference)? else {
            return Err(conflict());
        };
        return Ok(match font {
            SourceTableFontStyle::Reference { index, .. } => Ok(Input::Reference(*index)),
            SourceTableFontStyle::Collection { fonts, .. } => {
                match collection(fonts, slot, script, limits, check)? {
                    Ok((font, supplemental)) => Ok(Input::Font {
                        font,
                        table_font: Some(TableFontBinding { slot, supplemental }),
                    }),
                    Err(reason) => Err(reason),
                }
            }
        });
    }
    let node = cascade::declaration(index, reference)?;
    if let Some(at) = node.retained_ordinals.first() {
        return Ok(Err(TypefaceUnresolved::RetainedDeclaration {
            origin: reference.origin.at(*at),
        }));
    }
    Ok(Ok(match &node.value {
        SourceTextValue::Font { font } => {
            check_name(&font.typeface, limits)?;
            Input::Font {
                font: *font.clone(),
                table_font: None,
            }
        }
        SourceTextValue::FontReference { index } => Input::Reference(*index),
        _ => return Err(conflict()),
    }))
}
type SelectedFont = Result<(SourceTextFont, Option<u32>), TypefaceUnresolved>;
pub(super) fn collection(
    collection: &SourceFontCollection,
    slot: NativeFontSlot,
    script: Option<&str>,
    limits: TypefaceLimits,
    check: &dyn Fn() -> bool,
) -> Result<SelectedFont, PptxError> {
    cancelled(check)?;
    if slot == NativeFontSlot::Symbol {
        return Ok(Err(TypefaceUnresolved::SymbolThemeFont {}));
    }
    if let Some(font) = slot.font(collection).filter(|f| !f.typeface.is_empty()) {
        check_name(&font.typeface, limits)?;
        return Ok(Ok((font.clone(), None)));
    }
    let Some(script) = script else {
        return Ok(Err(TypefaceUnresolved::ScriptRequired {}));
    };
    if collection.supplemental.len() > limits.max_supplements {
        return Err(PptxError::Limit("theme supplemental fonts"));
    }
    let mut found = None;
    for (i, font) in collection.supplemental.iter().enumerate() {
        cancelled(check)?;
        if font.script == script {
            if found.is_some() {
                return Ok(Err(TypefaceUnresolved::AmbiguousSupplemental {
                    script: script.into(),
                }));
            }
            check_name(&font.typeface, limits)?;
            found = Some((
                SourceTextFont {
                    typeface: font.typeface.clone(),
                    panose: None,
                    pitch_family: None,
                    charset: None,
                },
                Some(i as u32),
            ));
        }
    }
    Ok(match found {
        None => Err(TypefaceUnresolved::MissingSupplemental {
            script: script.into(),
        }),
        Some((font, _)) if font.typeface.is_empty() => Err(TypefaceUnresolved::EmptyTypeface {}),
        Some(value) => Ok(value),
    })
}
