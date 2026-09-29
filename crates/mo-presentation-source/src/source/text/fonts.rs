//! Resolve native typeface declarations and theme references. Does not inspect
//! host fonts, map Unicode scripts to native slots or choose substitutions.
use super::{cascade::*, *};
use crate::{PptxError, cancelled, source::*, value};
use theme::{SourceFontCollection, SourceThemeSchemeRef};

mod selection;

pub const FONT_PROFILE: &str = "drawingml-explicit-theme-typeface-draft-v1";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NativeFontSlot {
    Latin,
    EastAsian,
    ComplexScript,
    Symbol,
}
impl NativeFontSlot {
    fn character(self) -> CharacterSlot {
        match self {
            Self::Latin => CharacterSlot::Latin,
            Self::EastAsian => CharacterSlot::EastAsian,
            Self::ComplexScript => CharacterSlot::ComplexScript,
            Self::Symbol => CharacterSlot::Symbol,
        }
    }
    fn font(self, collection: &SourceFontCollection) -> Option<&SourceTextFont> {
        match self {
            Self::Latin => collection.latin.as_ref(),
            Self::EastAsian => collection.east_asian.as_ref(),
            Self::ComplexScript => collection.complex_script.as_ref(),
            Self::Symbol => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeFontBinding {
    pub scheme: SourceThemeSchemeRef,
    pub collection: NativeFontCollectionIndex,
    pub slot: NativeFontSlot,
    /// Index into the original ordered supplemental list, not a physical ordinal.
    pub supplemental: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableFontBinding {
    pub slot: NativeFontSlot,
    /// Index into the table font collection's ordered supplemental list.
    pub supplemental: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTypeface {
    pub typeface: String,
    pub declared_by: TextStyleDeclaration,
    /// Script/collection location for an explicit table a:font declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_font: Option<TableFontBinding>,
    /// Original selected font declaration; absent for a fontRef fallback.
    pub authored_font: Option<SourceTextFont>,
    pub theme: Option<ThemeFontBinding>,
    /// Selected named theme font preserves its own metadata, independently of author hints.
    pub theme_font: Option<SourceTextFont>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TypefaceUnresolved {
    MissingDeclaration {},
    EmptyTypeface {},
    NoThemeFont {},
    DisabledThemeFont {},
    SymbolThemeFont {},
    ScriptRequired {},
    MissingSupplemental {
        script: String,
    },
    AmbiguousSupplemental {
        script: String,
    },
    UnknownThemeToken {
        token: String,
    },
    RetainedDeclaration {
        origin: TextStyleOrigin,
    },
    RetainedTheme {
        scheme: SourceThemeSchemeRef,
        source_ordinal: u32,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum TypefaceOutcome {
    Named { font: Box<NativeTypeface> },
    Unresolved { reason: TypefaceUnresolved },
}
#[derive(Debug, Clone, Copy)]
pub struct TypefaceLimits {
    pub max_name_bytes: usize,
    pub max_supplements: usize,
}
impl Default for TypefaceLimits {
    fn default() -> Self {
        Self {
            max_name_bytes: 4096,
            max_supplements: 4096,
        }
    }
}
fn conflict() -> PptxError {
    PptxError::SourceConflict("native typeface source binding differs".into())
}
fn unresolved(reason: TypefaceUnresolved) -> Result<TypefaceOutcome, PptxError> {
    Ok(TypefaceOutcome::Unresolved { reason })
}
fn check_name(s: &str, limits: TypefaceLimits) -> Result<(), PptxError> {
    if s.len() > limits.max_name_bytes {
        Err(PptxError::Limit("native font name bytes"))
    } else {
        Ok(())
    }
}
fn token(s: &str) -> Option<(NativeFontCollectionIndex, NativeFontSlot)> {
    use NativeFontCollectionIndex::{Major, Minor};
    use NativeFontSlot::*;
    Some(match s {
        "+mj-lt" => (Major, Latin),
        "+mj-ea" => (Major, EastAsian),
        "+mj-cs" => (Major, ComplexScript),
        "+mn-lt" => (Minor, Latin),
        "+mn-ea" => (Minor, EastAsian),
        "+mn-cs" => (Minor, ComplexScript),
        _ => return None,
    })
}

/// `script` is an explicit native theme key such as Hans/Hant/Jpan/Arab, not a
/// guessed locale or a font fallback list. The source compiler supplies it after
/// itemization and language/profile resolution. `run=None` selects end_style.
#[allow(clippy::too_many_arguments)]
pub fn resolve(
    index: &SourceIndex,
    text: &CascadedText,
    paragraph: u32,
    run: Option<u32>,
    slot: NativeFontSlot,
    script: Option<&str>,
    limits: TypefaceLimits,
    check: &dyn Fn() -> bool,
) -> Result<TypefaceOutcome, PptxError> {
    cancelled(check)?;
    if text.source_sha256 != index.source_sha256 || text.profile != PROFILE {
        return Err(conflict());
    }
    if script.is_some_and(|s| {
        s.len() != 4
            || !s.as_bytes()[0].is_ascii_uppercase()
            || !s.as_bytes()[1..].iter().all(u8::is_ascii_lowercase)
    }) {
        return Err(value(
            "typeface.script",
            "expected an explicit four-letter theme script key",
        ));
    }
    let surface = index.surfaces.get(&text.object.part).ok_or_else(conflict)?;
    if !surface
        .objects
        .iter()
        .any(|o| o.native_id == text.object.native_id)
    {
        return Err(conflict());
    }
    let p = text
        .paragraphs
        .get(paragraph as usize)
        .ok_or_else(|| value("typeface.paragraph", "out of range"))?;
    let style = match run {
        None => &p.end_style,
        Some(r) => {
            &p.runs
                .get(r as usize)
                .ok_or_else(|| value("typeface.run", "out of range"))?
                .style
        }
    };
    let Some(reference) = style
        .declarations
        .get(&slot.character())
        .or(text.font_reference.as_ref())
    else {
        return unresolved(TypefaceUnresolved::MissingDeclaration {});
    };
    let input = match selection::declaration(index, reference, slot, script, limits, check)? {
        Ok(input) => input,
        Err(reason) => return unresolved(reason),
    };
    let (authored, table_font, choice) = match input {
        selection::Input::Font { font, table_font } => {
            check_name(&font.typeface, limits)?;
            if font.typeface.is_empty() {
                return unresolved(TypefaceUnresolved::EmptyTypeface {});
            }
            if let Some(choice) = token(&font.typeface) {
                (Some(font), table_font, choice)
            } else {
                if font.typeface.starts_with("+mj-") || font.typeface.starts_with("+mn-") {
                    return unresolved(TypefaceUnresolved::UnknownThemeToken {
                        token: font.typeface,
                    });
                }
                return Ok(TypefaceOutcome::Named {
                    font: Box::new(NativeTypeface {
                        typeface: font.typeface.clone(),
                        declared_by: reference.clone(),
                        authored_font: Some(font),
                        table_font,
                        theme: None,
                        theme_font: None,
                    }),
                });
            }
        }
        selection::Input::Reference(collection) => {
            if collection == NativeFontCollectionIndex::None {
                return unresolved(TypefaceUnresolved::DisabledThemeFont {});
            }
            if slot == NativeFontSlot::Symbol {
                return unresolved(TypefaceUnresolved::SymbolThemeFont {});
            }
            (None, None, (collection, slot))
        }
    };
    let Some(selection) = &surface.theme_selection.fonts else {
        return unresolved(TypefaceUnresolved::NoThemeFont {});
    };
    let scheme = index
        .themes
        .get(&selection.part)
        .and_then(|t| t.font_scheme.as_ref())
        .filter(|s| s.source_ordinal == selection.source_ordinal)
        .ok_or_else(conflict)?;
    if let Some(at) = scheme.retained_ordinals.first() {
        return unresolved(TypefaceUnresolved::RetainedTheme {
            scheme: selection.clone(),
            source_ordinal: *at,
        });
    }
    let collection = match choice.0 {
        NativeFontCollectionIndex::Major => &scheme.major,
        NativeFontCollectionIndex::Minor => &scheme.minor,
        _ => return Err(conflict()),
    };
    let (font, supplemental) =
        match selection::collection(collection, choice.1, script, limits, check)? {
            Ok(value) => value,
            Err(reason) => return unresolved(reason),
        };
    if token(&font.typeface).is_some()
        || font.typeface.starts_with("+mj-")
        || font.typeface.starts_with("+mn-")
    {
        return unresolved(TypefaceUnresolved::UnknownThemeToken {
            token: font.typeface,
        });
    }
    cancelled(check)?;
    Ok(TypefaceOutcome::Named {
        font: Box::new(NativeTypeface {
            typeface: font.typeface.clone(),
            declared_by: reference.clone(),
            authored_font: authored,
            table_font,
            theme: Some(ThemeFontBinding {
                scheme: selection.clone(),
                collection: choice.0,
                slot: choice.1,
                supplemental,
            }),
            theme_font: Some(font),
        }),
    })
}
