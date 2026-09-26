//! Native enumeration facts from ECMA-376 Transitional dml-main.xsd.
//! Source: https://ecma-international.org/publications-and-standards/standards/ecma-376/
//! XSD SHA-256: 6978ba7e889070b0c3cb5b546b23e5a6c3516134afc53b87a21f482ca33f3858
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum SchemeColor {
    #[serde(rename = "bg1")]
    Bg1,
    #[serde(rename = "tx1")]
    Tx1,
    #[serde(rename = "bg2")]
    Bg2,
    #[serde(rename = "tx2")]
    Tx2,
    #[serde(rename = "accent1")]
    Accent1,
    #[serde(rename = "accent2")]
    Accent2,
    #[serde(rename = "accent3")]
    Accent3,
    #[serde(rename = "accent4")]
    Accent4,
    #[serde(rename = "accent5")]
    Accent5,
    #[serde(rename = "accent6")]
    Accent6,
    #[serde(rename = "hlink")]
    Hlink,
    #[serde(rename = "folHlink")]
    FolHlink,
    #[serde(rename = "phClr")]
    PhClr,
    #[serde(rename = "dk1")]
    Dk1,
    #[serde(rename = "lt1")]
    Lt1,
    #[serde(rename = "dk2")]
    Dk2,
    #[serde(rename = "lt2")]
    Lt2,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum ColorSlot {
    #[serde(rename = "dk1")]
    Dk1,
    #[serde(rename = "lt1")]
    Lt1,
    #[serde(rename = "dk2")]
    Dk2,
    #[serde(rename = "lt2")]
    Lt2,
    #[serde(rename = "accent1")]
    Accent1,
    #[serde(rename = "accent2")]
    Accent2,
    #[serde(rename = "accent3")]
    Accent3,
    #[serde(rename = "accent4")]
    Accent4,
    #[serde(rename = "accent5")]
    Accent5,
    #[serde(rename = "accent6")]
    Accent6,
    #[serde(rename = "hlink")]
    Hlink,
    #[serde(rename = "folHlink")]
    FolHlink,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum SystemColor {
    #[serde(rename = "scrollBar")]
    ScrollBar,
    #[serde(rename = "background")]
    Background,
    #[serde(rename = "activeCaption")]
    ActiveCaption,
    #[serde(rename = "inactiveCaption")]
    InactiveCaption,
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "windowFrame")]
    WindowFrame,
    #[serde(rename = "menuText")]
    MenuText,
    #[serde(rename = "windowText")]
    WindowText,
    #[serde(rename = "captionText")]
    CaptionText,
    #[serde(rename = "activeBorder")]
    ActiveBorder,
    #[serde(rename = "inactiveBorder")]
    InactiveBorder,
    #[serde(rename = "appWorkspace")]
    AppWorkspace,
    #[serde(rename = "highlight")]
    Highlight,
    #[serde(rename = "highlightText")]
    HighlightText,
    #[serde(rename = "btnFace")]
    BtnFace,
    #[serde(rename = "btnShadow")]
    BtnShadow,
    #[serde(rename = "grayText")]
    GrayText,
    #[serde(rename = "btnText")]
    BtnText,
    #[serde(rename = "inactiveCaptionText")]
    InactiveCaptionText,
    #[serde(rename = "btnHighlight")]
    BtnHighlight,
    #[serde(rename = "3dDkShadow")]
    ThreeDDarkShadow,
    #[serde(rename = "3dLight")]
    ThreeDLight,
    #[serde(rename = "infoText")]
    InfoText,
    #[serde(rename = "infoBk")]
    InfoBk,
    #[serde(rename = "hotLight")]
    HotLight,
    #[serde(rename = "gradientActiveCaption")]
    GradientActiveCaption,
    #[serde(rename = "gradientInactiveCaption")]
    GradientInactiveCaption,
    #[serde(rename = "menuHighlight")]
    MenuHighlight,
    #[serde(rename = "menuBar")]
    MenuBar,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum PresetColor {
    #[serde(rename = "aliceBlue")]
    AliceBlue,
    #[serde(rename = "antiqueWhite")]
    AntiqueWhite,
    #[serde(rename = "aqua")]
    Aqua,
    #[serde(rename = "aquamarine")]
    Aquamarine,
    #[serde(rename = "azure")]
    Azure,
    #[serde(rename = "beige")]
    Beige,
    #[serde(rename = "bisque")]
    Bisque,
    #[serde(rename = "black")]
    Black,
    #[serde(rename = "blanchedAlmond")]
    BlanchedAlmond,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "blueViolet")]
    BlueViolet,
    #[serde(rename = "brown")]
    Brown,
    #[serde(rename = "burlyWood")]
    BurlyWood,
    #[serde(rename = "cadetBlue")]
    CadetBlue,
    #[serde(rename = "chartreuse")]
    Chartreuse,
    #[serde(rename = "chocolate")]
    Chocolate,
    #[serde(rename = "coral")]
    Coral,
    #[serde(rename = "cornflowerBlue")]
    CornflowerBlue,
    #[serde(rename = "cornsilk")]
    Cornsilk,
    #[serde(rename = "crimson")]
    Crimson,
    #[serde(rename = "cyan")]
    Cyan,
    #[serde(rename = "darkBlue")]
    DarkBlue,
    #[serde(rename = "darkCyan")]
    DarkCyan,
    #[serde(rename = "darkGoldenrod")]
    DarkGoldenrod,
    #[serde(rename = "darkGray")]
    DarkGray,
    #[serde(rename = "darkGrey")]
    DarkGrey,
    #[serde(rename = "darkGreen")]
    DarkGreen,
    #[serde(rename = "darkKhaki")]
    DarkKhaki,
    #[serde(rename = "darkMagenta")]
    DarkMagenta,
    #[serde(rename = "darkOliveGreen")]
    DarkOliveGreen,
    #[serde(rename = "darkOrange")]
    DarkOrange,
    #[serde(rename = "darkOrchid")]
    DarkOrchid,
    #[serde(rename = "darkRed")]
    DarkRed,
    #[serde(rename = "darkSalmon")]
    DarkSalmon,
    #[serde(rename = "darkSeaGreen")]
    DarkSeaGreen,
    #[serde(rename = "darkSlateBlue")]
    DarkSlateBlue,
    #[serde(rename = "darkSlateGray")]
    DarkSlateGray,
    #[serde(rename = "darkSlateGrey")]
    DarkSlateGrey,
    #[serde(rename = "darkTurquoise")]
    DarkTurquoise,
    #[serde(rename = "darkViolet")]
    DarkViolet,
    #[serde(rename = "dkBlue")]
    DkBlue,
    #[serde(rename = "dkCyan")]
    DkCyan,
    #[serde(rename = "dkGoldenrod")]
    DkGoldenrod,
    #[serde(rename = "dkGray")]
    DkGray,
    #[serde(rename = "dkGrey")]
    DkGrey,
    #[serde(rename = "dkGreen")]
    DkGreen,
    #[serde(rename = "dkKhaki")]
    DkKhaki,
    #[serde(rename = "dkMagenta")]
    DkMagenta,
    #[serde(rename = "dkOliveGreen")]
    DkOliveGreen,
    #[serde(rename = "dkOrange")]
    DkOrange,
    #[serde(rename = "dkOrchid")]
    DkOrchid,
    #[serde(rename = "dkRed")]
    DkRed,
    #[serde(rename = "dkSalmon")]
    DkSalmon,
    #[serde(rename = "dkSeaGreen")]
    DkSeaGreen,
    #[serde(rename = "dkSlateBlue")]
    DkSlateBlue,
    #[serde(rename = "dkSlateGray")]
    DkSlateGray,
    #[serde(rename = "dkSlateGrey")]
    DkSlateGrey,
    #[serde(rename = "dkTurquoise")]
    DkTurquoise,
    #[serde(rename = "dkViolet")]
    DkViolet,
    #[serde(rename = "deepPink")]
    DeepPink,
    #[serde(rename = "deepSkyBlue")]
    DeepSkyBlue,
    #[serde(rename = "dimGray")]
    DimGray,
    #[serde(rename = "dimGrey")]
    DimGrey,
    #[serde(rename = "dodgerBlue")]
    DodgerBlue,
    #[serde(rename = "firebrick")]
    Firebrick,
    #[serde(rename = "floralWhite")]
    FloralWhite,
    #[serde(rename = "forestGreen")]
    ForestGreen,
    #[serde(rename = "fuchsia")]
    Fuchsia,
    #[serde(rename = "gainsboro")]
    Gainsboro,
    #[serde(rename = "ghostWhite")]
    GhostWhite,
    #[serde(rename = "gold")]
    Gold,
    #[serde(rename = "goldenrod")]
    Goldenrod,
    #[serde(rename = "gray")]
    Gray,
    #[serde(rename = "grey")]
    Grey,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "greenYellow")]
    GreenYellow,
    #[serde(rename = "honeydew")]
    Honeydew,
    #[serde(rename = "hotPink")]
    HotPink,
    #[serde(rename = "indianRed")]
    IndianRed,
    #[serde(rename = "indigo")]
    Indigo,
    #[serde(rename = "ivory")]
    Ivory,
    #[serde(rename = "khaki")]
    Khaki,
    #[serde(rename = "lavender")]
    Lavender,
    #[serde(rename = "lavenderBlush")]
    LavenderBlush,
    #[serde(rename = "lawnGreen")]
    LawnGreen,
    #[serde(rename = "lemonChiffon")]
    LemonChiffon,
    #[serde(rename = "lightBlue")]
    LightBlue,
    #[serde(rename = "lightCoral")]
    LightCoral,
    #[serde(rename = "lightCyan")]
    LightCyan,
    #[serde(rename = "lightGoldenrodYellow")]
    LightGoldenrodYellow,
    #[serde(rename = "lightGray")]
    LightGray,
    #[serde(rename = "lightGrey")]
    LightGrey,
    #[serde(rename = "lightGreen")]
    LightGreen,
    #[serde(rename = "lightPink")]
    LightPink,
    #[serde(rename = "lightSalmon")]
    LightSalmon,
    #[serde(rename = "lightSeaGreen")]
    LightSeaGreen,
    #[serde(rename = "lightSkyBlue")]
    LightSkyBlue,
    #[serde(rename = "lightSlateGray")]
    LightSlateGray,
    #[serde(rename = "lightSlateGrey")]
    LightSlateGrey,
    #[serde(rename = "lightSteelBlue")]
    LightSteelBlue,
    #[serde(rename = "lightYellow")]
    LightYellow,
    #[serde(rename = "ltBlue")]
    LtBlue,
    #[serde(rename = "ltCoral")]
    LtCoral,
    #[serde(rename = "ltCyan")]
    LtCyan,
    #[serde(rename = "ltGoldenrodYellow")]
    LtGoldenrodYellow,
    #[serde(rename = "ltGray")]
    LtGray,
    #[serde(rename = "ltGrey")]
    LtGrey,
    #[serde(rename = "ltGreen")]
    LtGreen,
    #[serde(rename = "ltPink")]
    LtPink,
    #[serde(rename = "ltSalmon")]
    LtSalmon,
    #[serde(rename = "ltSeaGreen")]
    LtSeaGreen,
    #[serde(rename = "ltSkyBlue")]
    LtSkyBlue,
    #[serde(rename = "ltSlateGray")]
    LtSlateGray,
    #[serde(rename = "ltSlateGrey")]
    LtSlateGrey,
    #[serde(rename = "ltSteelBlue")]
    LtSteelBlue,
    #[serde(rename = "ltYellow")]
    LtYellow,
    #[serde(rename = "lime")]
    Lime,
    #[serde(rename = "limeGreen")]
    LimeGreen,
    #[serde(rename = "linen")]
    Linen,
    #[serde(rename = "magenta")]
    Magenta,
    #[serde(rename = "maroon")]
    Maroon,
    #[serde(rename = "medAquamarine")]
    MedAquamarine,
    #[serde(rename = "medBlue")]
    MedBlue,
    #[serde(rename = "medOrchid")]
    MedOrchid,
    #[serde(rename = "medPurple")]
    MedPurple,
    #[serde(rename = "medSeaGreen")]
    MedSeaGreen,
    #[serde(rename = "medSlateBlue")]
    MedSlateBlue,
    #[serde(rename = "medSpringGreen")]
    MedSpringGreen,
    #[serde(rename = "medTurquoise")]
    MedTurquoise,
    #[serde(rename = "medVioletRed")]
    MedVioletRed,
    #[serde(rename = "mediumAquamarine")]
    MediumAquamarine,
    #[serde(rename = "mediumBlue")]
    MediumBlue,
    #[serde(rename = "mediumOrchid")]
    MediumOrchid,
    #[serde(rename = "mediumPurple")]
    MediumPurple,
    #[serde(rename = "mediumSeaGreen")]
    MediumSeaGreen,
    #[serde(rename = "mediumSlateBlue")]
    MediumSlateBlue,
    #[serde(rename = "mediumSpringGreen")]
    MediumSpringGreen,
    #[serde(rename = "mediumTurquoise")]
    MediumTurquoise,
    #[serde(rename = "mediumVioletRed")]
    MediumVioletRed,
    #[serde(rename = "midnightBlue")]
    MidnightBlue,
    #[serde(rename = "mintCream")]
    MintCream,
    #[serde(rename = "mistyRose")]
    MistyRose,
    #[serde(rename = "moccasin")]
    Moccasin,
    #[serde(rename = "navajoWhite")]
    NavajoWhite,
    #[serde(rename = "navy")]
    Navy,
    #[serde(rename = "oldLace")]
    OldLace,
    #[serde(rename = "olive")]
    Olive,
    #[serde(rename = "oliveDrab")]
    OliveDrab,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "orangeRed")]
    OrangeRed,
    #[serde(rename = "orchid")]
    Orchid,
    #[serde(rename = "paleGoldenrod")]
    PaleGoldenrod,
    #[serde(rename = "paleGreen")]
    PaleGreen,
    #[serde(rename = "paleTurquoise")]
    PaleTurquoise,
    #[serde(rename = "paleVioletRed")]
    PaleVioletRed,
    #[serde(rename = "papayaWhip")]
    PapayaWhip,
    #[serde(rename = "peachPuff")]
    PeachPuff,
    #[serde(rename = "peru")]
    Peru,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "powderBlue")]
    PowderBlue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "rosyBrown")]
    RosyBrown,
    #[serde(rename = "royalBlue")]
    RoyalBlue,
    #[serde(rename = "saddleBrown")]
    SaddleBrown,
    #[serde(rename = "salmon")]
    Salmon,
    #[serde(rename = "sandyBrown")]
    SandyBrown,
    #[serde(rename = "seaGreen")]
    SeaGreen,
    #[serde(rename = "seaShell")]
    SeaShell,
    #[serde(rename = "sienna")]
    Sienna,
    #[serde(rename = "silver")]
    Silver,
    #[serde(rename = "skyBlue")]
    SkyBlue,
    #[serde(rename = "slateBlue")]
    SlateBlue,
    #[serde(rename = "slateGray")]
    SlateGray,
    #[serde(rename = "slateGrey")]
    SlateGrey,
    #[serde(rename = "snow")]
    Snow,
    #[serde(rename = "springGreen")]
    SpringGreen,
    #[serde(rename = "steelBlue")]
    SteelBlue,
    #[serde(rename = "tan")]
    Tan,
    #[serde(rename = "teal")]
    Teal,
    #[serde(rename = "thistle")]
    Thistle,
    #[serde(rename = "tomato")]
    Tomato,
    #[serde(rename = "turquoise")]
    Turquoise,
    #[serde(rename = "violet")]
    Violet,
    #[serde(rename = "wheat")]
    Wheat,
    #[serde(rename = "white")]
    White,
    #[serde(rename = "whiteSmoke")]
    WhiteSmoke,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "yellowGreen")]
    YellowGreen,
}
