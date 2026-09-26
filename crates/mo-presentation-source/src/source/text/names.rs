//! Native enumeration facts from ECMA-376 Transitional dml-main.xsd.
//! Source: https://ecma-international.org/publications-and-standards/standards/ecma-376/
//! XSD SHA-256: 6978ba7e889070b0c3cb5b546b23e5a6c3516134afc53b87a21f482ca33f3858
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextAnchor {
    #[serde(rename = "t")]
    T,
    #[serde(rename = "ctr")]
    Ctr,
    #[serde(rename = "b")]
    B,
    #[serde(rename = "just")]
    Just,
    #[serde(rename = "dist")]
    Dist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextVerticalOverflow {
    #[serde(rename = "overflow")]
    Overflow,
    #[serde(rename = "ellipsis")]
    Ellipsis,
    #[serde(rename = "clip")]
    Clip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextHorizontalOverflow {
    #[serde(rename = "overflow")]
    Overflow,
    #[serde(rename = "clip")]
    Clip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextVertical {
    #[serde(rename = "horz")]
    Horz,
    #[serde(rename = "vert")]
    Vert,
    #[serde(rename = "vert270")]
    Vert270,
    #[serde(rename = "wordArtVert")]
    WordArtVert,
    #[serde(rename = "eaVert")]
    EaVert,
    #[serde(rename = "mongolianVert")]
    MongolianVert,
    #[serde(rename = "wordArtVertRtl")]
    WordArtVertRtl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextWrap {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "square")]
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextUnderline {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "words")]
    Words,
    #[serde(rename = "sng")]
    Sng,
    #[serde(rename = "dbl")]
    Dbl,
    #[serde(rename = "heavy")]
    Heavy,
    #[serde(rename = "dotted")]
    Dotted,
    #[serde(rename = "dottedHeavy")]
    DottedHeavy,
    #[serde(rename = "dash")]
    Dash,
    #[serde(rename = "dashHeavy")]
    DashHeavy,
    #[serde(rename = "dashLong")]
    DashLong,
    #[serde(rename = "dashLongHeavy")]
    DashLongHeavy,
    #[serde(rename = "dotDash")]
    DotDash,
    #[serde(rename = "dotDashHeavy")]
    DotDashHeavy,
    #[serde(rename = "dotDotDash")]
    DotDotDash,
    #[serde(rename = "dotDotDashHeavy")]
    DotDotDashHeavy,
    #[serde(rename = "wavy")]
    Wavy,
    #[serde(rename = "wavyHeavy")]
    WavyHeavy,
    #[serde(rename = "wavyDbl")]
    WavyDbl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextStrike {
    #[serde(rename = "noStrike")]
    NoStrike,
    #[serde(rename = "sngStrike")]
    SngStrike,
    #[serde(rename = "dblStrike")]
    DblStrike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextCaps {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextAlign {
    #[serde(rename = "l")]
    L,
    #[serde(rename = "ctr")]
    Ctr,
    #[serde(rename = "r")]
    R,
    #[serde(rename = "just")]
    Just,
    #[serde(rename = "justLow")]
    JustLow,
    #[serde(rename = "dist")]
    Dist,
    #[serde(rename = "thaiDist")]
    ThaiDist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextFontAlign {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "t")]
    T,
    #[serde(rename = "ctr")]
    Ctr,
    #[serde(rename = "base")]
    Base,
    #[serde(rename = "b")]
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextTabAlign {
    #[serde(rename = "l")]
    L,
    #[serde(rename = "ctr")]
    Ctr,
    #[serde(rename = "r")]
    R,
    #[serde(rename = "dec")]
    Dec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextAutonumber {
    #[serde(rename = "alphaLcParenBoth")]
    AlphaLcParenBoth,
    #[serde(rename = "alphaUcParenBoth")]
    AlphaUcParenBoth,
    #[serde(rename = "alphaLcParenR")]
    AlphaLcParenR,
    #[serde(rename = "alphaUcParenR")]
    AlphaUcParenR,
    #[serde(rename = "alphaLcPeriod")]
    AlphaLcPeriod,
    #[serde(rename = "alphaUcPeriod")]
    AlphaUcPeriod,
    #[serde(rename = "arabicParenBoth")]
    ArabicParenBoth,
    #[serde(rename = "arabicParenR")]
    ArabicParenR,
    #[serde(rename = "arabicPeriod")]
    ArabicPeriod,
    #[serde(rename = "arabicPlain")]
    ArabicPlain,
    #[serde(rename = "romanLcParenBoth")]
    RomanLcParenBoth,
    #[serde(rename = "romanUcParenBoth")]
    RomanUcParenBoth,
    #[serde(rename = "romanLcParenR")]
    RomanLcParenR,
    #[serde(rename = "romanUcParenR")]
    RomanUcParenR,
    #[serde(rename = "romanLcPeriod")]
    RomanLcPeriod,
    #[serde(rename = "romanUcPeriod")]
    RomanUcPeriod,
    #[serde(rename = "circleNumDbPlain")]
    CircleNumDbPlain,
    #[serde(rename = "circleNumWdBlackPlain")]
    CircleNumWdBlackPlain,
    #[serde(rename = "circleNumWdWhitePlain")]
    CircleNumWdWhitePlain,
    #[serde(rename = "arabicDbPeriod")]
    ArabicDbPeriod,
    #[serde(rename = "arabicDbPlain")]
    ArabicDbPlain,
    #[serde(rename = "ea1ChsPeriod")]
    Ea1ChsPeriod,
    #[serde(rename = "ea1ChsPlain")]
    Ea1ChsPlain,
    #[serde(rename = "ea1ChtPeriod")]
    Ea1ChtPeriod,
    #[serde(rename = "ea1ChtPlain")]
    Ea1ChtPlain,
    #[serde(rename = "ea1JpnChsDbPeriod")]
    Ea1JpnChsDbPeriod,
    #[serde(rename = "ea1JpnKorPlain")]
    Ea1JpnKorPlain,
    #[serde(rename = "ea1JpnKorPeriod")]
    Ea1JpnKorPeriod,
    #[serde(rename = "arabic1Minus")]
    Arabic1Minus,
    #[serde(rename = "arabic2Minus")]
    Arabic2Minus,
    #[serde(rename = "hebrew2Minus")]
    Hebrew2Minus,
    #[serde(rename = "thaiAlphaPeriod")]
    ThaiAlphaPeriod,
    #[serde(rename = "thaiAlphaParenR")]
    ThaiAlphaParenR,
    #[serde(rename = "thaiAlphaParenBoth")]
    ThaiAlphaParenBoth,
    #[serde(rename = "thaiNumPeriod")]
    ThaiNumPeriod,
    #[serde(rename = "thaiNumParenR")]
    ThaiNumParenR,
    #[serde(rename = "thaiNumParenBoth")]
    ThaiNumParenBoth,
    #[serde(rename = "hindiAlphaPeriod")]
    HindiAlphaPeriod,
    #[serde(rename = "hindiNumPeriod")]
    HindiNumPeriod,
    #[serde(rename = "hindiNumParenR")]
    HindiNumParenR,
    #[serde(rename = "hindiAlpha1Period")]
    HindiAlpha1Period,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeFontCollectionIndex {
    #[serde(rename = "major")]
    Major,
    #[serde(rename = "minor")]
    Minor,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeTextElement {
    #[serde(rename = "txBody")]
    TxBody,
    #[serde(rename = "txStyles")]
    TxStyles,
    #[serde(rename = "defaultTextStyle")]
    DefaultTextStyle,
    #[serde(rename = "titleStyle")]
    TitleStyle,
    #[serde(rename = "bodyStyle")]
    BodyStyle,
    #[serde(rename = "otherStyle")]
    OtherStyle,
    #[serde(rename = "lstStyle")]
    LstStyle,
    #[serde(rename = "bodyPr")]
    BodyPr,
    #[serde(rename = "p")]
    P,
    #[serde(rename = "pPr")]
    PPr,
    #[serde(rename = "defPPr")]
    DefPPr,
    #[serde(rename = "lvl1pPr")]
    Lvl1pPr,
    #[serde(rename = "lvl2pPr")]
    Lvl2pPr,
    #[serde(rename = "lvl3pPr")]
    Lvl3pPr,
    #[serde(rename = "lvl4pPr")]
    Lvl4pPr,
    #[serde(rename = "lvl5pPr")]
    Lvl5pPr,
    #[serde(rename = "lvl6pPr")]
    Lvl6pPr,
    #[serde(rename = "lvl7pPr")]
    Lvl7pPr,
    #[serde(rename = "lvl8pPr")]
    Lvl8pPr,
    #[serde(rename = "lvl9pPr")]
    Lvl9pPr,
    #[serde(rename = "r")]
    R,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "fld")]
    Fld,
    #[serde(rename = "t")]
    T,
    #[serde(rename = "rPr")]
    RPr,
    #[serde(rename = "defRPr")]
    DefRPr,
    #[serde(rename = "endParaRPr")]
    EndParaRPr,
    #[serde(rename = "noAutofit")]
    NoAutofit,
    #[serde(rename = "normAutofit")]
    NormAutofit,
    #[serde(rename = "spAutoFit")]
    SpAutoFit,
    #[serde(rename = "lnSpc")]
    LnSpc,
    #[serde(rename = "spcBef")]
    SpcBef,
    #[serde(rename = "spcAft")]
    SpcAft,
    #[serde(rename = "spcPct")]
    SpcPct,
    #[serde(rename = "spcPts")]
    SpcPts,
    #[serde(rename = "buClrTx")]
    BuClrTx,
    #[serde(rename = "buClr")]
    BuClr,
    #[serde(rename = "buSzTx")]
    BuSzTx,
    #[serde(rename = "buSzPct")]
    BuSzPct,
    #[serde(rename = "buSzPts")]
    BuSzPts,
    #[serde(rename = "buFontTx")]
    BuFontTx,
    #[serde(rename = "buFont")]
    BuFont,
    #[serde(rename = "buNone")]
    BuNone,
    #[serde(rename = "buAutoNum")]
    BuAutoNum,
    #[serde(rename = "buChar")]
    BuChar,
    #[serde(rename = "tabLst")]
    TabLst,
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "latin")]
    Latin,
    #[serde(rename = "ea")]
    Ea,
    #[serde(rename = "cs")]
    Cs,
    #[serde(rename = "sym")]
    Sym,
    #[serde(rename = "fontRef")]
    FontRef,
    #[serde(rename = "highlight")]
    Highlight,
    #[serde(rename = "uLnTx")]
    ULnTx,
    #[serde(rename = "uLn")]
    ULn,
    #[serde(rename = "uFillTx")]
    UFillTx,
    #[serde(rename = "uFill")]
    UFill,
    #[serde(rename = "hlinkClick")]
    HlinkClick,
    #[serde(rename = "hlinkMouseOver")]
    HlinkMouseOver,
    #[serde(rename = "rtl")]
    Rtl,
    #[serde(rename = "noFill")]
    NoFill,
    #[serde(rename = "solidFill")]
    SolidFill,
    #[serde(rename = "gradFill")]
    GradFill,
    #[serde(rename = "blipFill")]
    BlipFill,
    #[serde(rename = "pattFill")]
    PattFill,
    #[serde(rename = "grpFill")]
    GrpFill,
    #[serde(rename = "ln")]
    Ln,
    #[serde(rename = "effectLst")]
    EffectLst,
    #[serde(rename = "effectDag")]
    EffectDag,
    #[serde(rename = "srgbClr")]
    SrgbClr,
    #[serde(rename = "scrgbClr")]
    ScrgbClr,
    #[serde(rename = "hslClr")]
    HslClr,
    #[serde(rename = "sysClr")]
    SysClr,
    #[serde(rename = "schemeClr")]
    SchemeClr,
    #[serde(rename = "prstClr")]
    PrstClr,
}
