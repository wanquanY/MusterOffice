//! Resolve native typeface declarations and theme references. Does not inspect
//! host fonts, map Unicode scripts to native slots or choose substitutions.
use super::{cascade::*, *};
use crate::{PptxError, cancelled, source::*, value};
use theme::{SourceFontCollection, SourceThemeSchemeRef};

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
pub struct NativeTypeface {
    pub typeface: String,
    pub declared_by: TextStyleDeclaration,
    /// Original run font declaration; absent for a shape fontRef fallback.
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
    let (reference, authored, choice) =
        if let Some(reference) = style.declarations.get(&slot.character()) {
            let node = declaration(index, reference)?;
            if let Some(at) = node.retained_ordinals.first() {
                return unresolved(TypefaceUnresolved::RetainedDeclaration {
                    origin: reference.origin.at(*at),
                });
            }
            let SourceTextValue::Font { font } = &node.value else {
                return Err(conflict());
            };
            check_name(&font.typeface, limits)?;
            if font.typeface.is_empty() {
                return unresolved(TypefaceUnresolved::EmptyTypeface {});
            }
            let choice = token(&font.typeface);
            if choice.is_none() {
                if font.typeface.starts_with("+mj-") || font.typeface.starts_with("+mn-") {
                    return unresolved(TypefaceUnresolved::UnknownThemeToken {
                        token: font.typeface.clone(),
                    });
                }
                return Ok(TypefaceOutcome::Named {
                    font: Box::new(NativeTypeface {
                        typeface: font.typeface.clone(),
                        declared_by: reference.clone(),
                        authored_font: Some(*font.clone()),
                        theme: None,
                        theme_font: None,
                    }),
                });
            }
            (reference, Some(*font.clone()), choice.expect("theme token"))
        } else if let Some(reference) = &text.font_reference {
            let node = declaration(index, reference)?;
            if let Some(at) = node.retained_ordinals.first() {
                return unresolved(TypefaceUnresolved::RetainedDeclaration {
                    origin: reference.origin.at(*at),
                });
            }
            let SourceTextValue::FontReference { index } = node.value else {
                return Err(conflict());
            };
            if index == NativeFontCollectionIndex::None {
                return unresolved(TypefaceUnresolved::DisabledThemeFont {});
            }
            if slot == NativeFontSlot::Symbol {
                return unresolved(TypefaceUnresolved::SymbolThemeFont {});
            }
            (reference, None, (index, slot))
        } else {
            return unresolved(TypefaceUnresolved::MissingDeclaration {});
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
    let font = choice.1.font(collection);
    let (font, supplemental) = if let Some(font) = font.filter(|f| !f.typeface.is_empty()) {
        check_name(&font.typeface, limits)?;
        (font.clone(), None)
    } else {
        let Some(script) = script else {
            return unresolved(TypefaceUnresolved::ScriptRequired {});
        };
        if collection.supplemental.len() > limits.max_supplements {
            return Err(PptxError::Limit("theme supplemental fonts"));
        }
        let mut found = None;
        for (i, font) in collection.supplemental.iter().enumerate() {
            cancelled(check)?;
            if font.script == script {
                if found.is_some() {
                    return unresolved(TypefaceUnresolved::AmbiguousSupplemental {
                        script: script.into(),
                    });
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
        let Some(value) = found else {
            return unresolved(TypefaceUnresolved::MissingSupplemental {
                script: script.into(),
            });
        };
        if value.0.typeface.is_empty() {
            return unresolved(TypefaceUnresolved::EmptyTypeface {});
        }
        value
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
