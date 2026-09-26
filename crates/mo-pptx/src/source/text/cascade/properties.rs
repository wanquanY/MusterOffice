use super::*;

macro_rules! fields {
    ($property:ident, $ty:ident, $merge:ident, {$($field:ident => $variant:ident),* $(,)?}) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub enum $property { $($variant),* }
        #[allow(clippy::clone_on_copy)]
        pub(super) fn $merge(target: &mut $ty, from: &$ty, origins: &mut BTreeMap<$property, TextStyleOrigin>, origin: &TextStyleOrigin, budget: &mut Budget<'_>) -> Result<(), PptxError> {
            $(if target.$field.is_none() && from.$field.is_some() {
                budget.bytes(origin.bytes())?;
                target.$field = from.$field.clone();
                origins.insert($property::$variant, origin.clone());
            })*
            Ok(())
        }
    }
}
fields!(ParagraphProperty, SourceTextParagraphAttributes, paragraph, {
    left_margin => LeftMargin, right_margin => RightMargin, level => Level, indent => Indent,
    alignment => Alignment, default_tab_size => DefaultTabSize, right_to_left => RightToLeft,
    east_asian_line_break => EastAsianLineBreak, font_alignment => FontAlignment,
    latin_line_break => LatinLineBreak, hanging_punctuation => HangingPunctuation
});
fields!(CharacterProperty, SourceTextCharacterAttributes, character, {
    kumimoji => Kumimoji, language => Language, alternative_language => AlternativeLanguage,
    size => Size, bold => Bold, italic => Italic, underline => Underline, strike => Strike,
    kerning => Kerning, caps => Caps, spacing => Spacing, normalize_height => NormalizeHeight,
    baseline => Baseline, no_proof => NoProof, dirty => Dirty, error => Error,
    smart_clean => SmartClean, smart_id => SmartId, bookmark => Bookmark
});

pub(super) fn paragraph_defaults() -> SourceTextParagraphAttributes {
    SourceTextParagraphAttributes {
        left_margin: Some(0),
        right_margin: Some(0),
        level: Some(0),
        indent: Some(0),
        alignment: Some(NativeTextAlign::L),
        default_tab_size: Some(
            NativeCoordinate::try_from("914400".to_owned()).expect("profile coordinate"),
        ),
        right_to_left: Some(false),
        east_asian_line_break: Some(true),
        font_alignment: Some(NativeTextFontAlign::Base),
        latin_line_break: Some(false),
        hanging_punctuation: Some(true),
    }
}
pub(super) fn character_defaults() -> SourceTextCharacterAttributes {
    SourceTextCharacterAttributes {
        kumimoji: Some(false),
        language: None,
        alternative_language: None,
        size: Some(1800),
        bold: Some(false),
        italic: Some(false),
        underline: Some(NativeTextUnderline::None),
        strike: Some(NativeTextStrike::NoStrike),
        kerning: None,
        caps: Some(NativeTextCaps::None),
        spacing: Some(NativeTextPoint::HundredthPoints { value: 0 }),
        normalize_height: Some(false),
        baseline: Some(NativePercentage::try_from("0".to_owned()).expect("profile percent")),
        no_proof: Some(false),
        dirty: Some(true),
        error: Some(false),
        smart_clean: Some(true),
        smart_id: Some(0),
        bookmark: None,
    }
}
pub(super) fn character_bytes(
    a: &SourceTextCharacterAttributes,
    budget: &mut Budget<'_>,
) -> Result<(), PptxError> {
    for s in [&a.language, &a.alternative_language, &a.bookmark]
        .into_iter()
        .flatten()
    {
        budget.bytes(s.len())?;
    }
    if let Some(p) = &a.baseline {
        budget.bytes(p.lexical().len())?;
    }
    if let Some(NativeTextPoint::UniversalMeasure { value }) = &a.spacing {
        budget.bytes(value.lexical().len())?;
    }
    Ok(())
}
pub(super) fn paragraph_slot(e: NativeTextElement) -> Option<ParagraphSlot> {
    use NativeTextElement as N;
    Some(match e {
        N::LnSpc => ParagraphSlot::LineSpacing,
        N::SpcBef => ParagraphSlot::SpaceBefore,
        N::SpcAft => ParagraphSlot::SpaceAfter,
        N::BuClr | N::BuClrTx => ParagraphSlot::BulletColor,
        N::BuSzTx | N::BuSzPts | N::BuSzPct => ParagraphSlot::BulletSize,
        N::BuFont | N::BuFontTx => ParagraphSlot::BulletFont,
        N::BuNone | N::BuChar | N::BuAutoNum => ParagraphSlot::Bullet,
        N::TabLst => ParagraphSlot::Tabs,
        _ => return None,
    })
}
pub(super) fn character_slot(e: NativeTextElement) -> Option<CharacterSlot> {
    use NativeTextElement as N;
    Some(match e {
        N::Ln => CharacterSlot::Line,
        N::NoFill | N::SolidFill | N::GradFill | N::BlipFill | N::PattFill | N::GrpFill => {
            CharacterSlot::Fill
        }
        N::EffectLst | N::EffectDag => CharacterSlot::Effects,
        N::Highlight => CharacterSlot::Highlight,
        N::ULnTx | N::ULn => CharacterSlot::UnderlineLine,
        N::UFillTx | N::UFill => CharacterSlot::UnderlineFill,
        N::Latin => CharacterSlot::Latin,
        N::Ea => CharacterSlot::EastAsian,
        N::Cs => CharacterSlot::ComplexScript,
        N::Sym => CharacterSlot::Symbol,
        N::HlinkClick => CharacterSlot::Click,
        N::HlinkMouseOver => CharacterSlot::MouseOver,
        N::Rtl => CharacterSlot::RightToLeft,
        _ => return None,
    })
}
