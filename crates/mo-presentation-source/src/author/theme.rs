use super::*;
use crate::source::{SourceCompatibility, theme::*};
use mo_presentation_model::{CharacterStyle, Color, Inherited, Theme};

pub(super) fn build(
    document: &Document,
    theme: Option<&Theme>,
    defaults: &ExportDefaults,
    identity: &Digest,
    ord: &mut Ordinals,
) -> Result<SourceThemePart, PptxError> {
    let name = theme.map_or("MusterOffice", |t| &t.name).to_owned();
    let family = match theme {
        Some(theme) => {
            let mut style = theme.default_text.clone();
            style.font = Inherited::Inherit;
            if style != CharacterStyle::default() {
                return Err(PptxError::Unsupported(
                    "theme-wide character defaults other than font need native style binding"
                        .into(),
                ));
            }
            match &theme.default_text.font {
                Inherited::Value(id) => &document.fonts[id].family,
                Inherited::Inherit => &defaults.font_family,
            }
        }
        None => &defaults.font_family,
    };
    let color_ordinal = ord.next()?;
    let mut colors = BTreeMap::new();
    for (slot, name) in COLOR_SLOTS {
        let rgba = theme
            .and_then(|t| t.colors.get(&slot))
            .or_else(|| defaults.theme_colors.get(&slot))
            .ok_or_else(|| value_error("defaults.themeColors", "missing theme color"))?;
        colors.insert(
            native(name.into())?,
            paint::color(&Color::Srgb { rgba: *rgba }, ord)?,
        );
    }
    let collection = SourceFontCollection {
        latin: Some(text::font(family)?),
        east_asian: Some(text::font(family)?),
        complex_script: Some(text::font(family)?),
        supplemental: vec![],
    };
    Ok(SourceThemePart {
        text_defaults: None,
        sha256: identity.clone(),
        kind: SourceThemeKind::Theme,
        name: Some(name.clone()),
        color_scheme: Some(SourceColorScheme {
            source_ordinal: color_ordinal,
            name: name.clone(),
            colors,
        }),
        font_scheme: Some(SourceFontScheme {
            source_ordinal: ord.next()?,
            retained_ordinals: vec![],
            name,
            major: collection.clone(),
            minor: collection,
        }),
        format_scheme: Some(format_scheme(ord)?),
        compatibility: SourceCompatibility::default(),
        notices: vec![],
        effect_nodes: BTreeMap::new(),
    })
}

fn format_scheme(ord: &mut Ordinals) -> Result<SourceFormatScheme, PptxError> {
    use crate::source::{effects::SourceEffectStyle, fill::*, line::*};
    let mut out = SourceFormatScheme {
        source_ordinal: ord.next()?,
        name: Some("MusterOffice".into()),
        fills: vec![],
        lines: vec![],
        effects: vec![],
        background_fills: vec![],
    };
    fn color(ord: &mut Ordinals) -> Result<SourceColor, PptxError> {
        Ok(SourceColor {
            source_ordinal: ord.next()?,
            value: SourceColorValue::Scheme {
                slot: SchemeColor::PhClr,
            },
            transforms: vec![],
        })
    }
    for entries in [&mut out.fills, &mut out.background_fills] {
        for _ in 0..3 {
            let id = ord.next()?;
            entries.push(SourceStyleEntry {
                source_ordinal: id,
                local_name: "solidFill".into(),
                fill: Some(SourceFill {
                    source_ordinal: id,
                    definition: SourceFillDefinition::Solid {
                        color: Some(color(ord)?),
                    },
                    retained_ordinals: vec![],
                }),
                line: None,
                effect_style: None,
            });
        }
    }
    for width in [12700, 25400, 38100] {
        let id = ord.next()?;
        out.lines.push(SourceStyleEntry {
            source_ordinal: id,
            local_name: "ln".into(),
            fill: None,
            effect_style: None,
            line: Some(SourceLine {
                source_ordinal: id,
                width: Some(mo_common::Emu::new(width)),
                cap: None,
                compound: None,
                alignment: None,
                fill: Some(SourceLineFill::Solid {
                    source_ordinal: ord.next()?,
                    color: Some(color(ord)?),
                }),
                dash: Some(SourceLineDash::Preset {
                    source_ordinal: ord.next()?,
                    value: Some(NativePresetDash::Solid),
                }),
                join: None,
                head: None,
                tail: None,
                retained_ordinals: vec![],
            }),
        });
    }
    for _ in 0..3 {
        let id = ord.next()?;
        out.effects.push(SourceStyleEntry {
            source_ordinal: id,
            local_name: "effectStyle".into(),
            fill: None,
            line: None,
            effect_style: Some(SourceEffectStyle {
                source_ordinal: id,
                effects: surface::empty_effects(ord)?,
                retained_ordinals: vec![],
            }),
        });
    }
    Ok(out)
}
