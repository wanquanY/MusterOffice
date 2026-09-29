/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { SourceColor, SourceColorMap, SourceDashStop, SourceEffectProperties, SourceFill, SourceFillBlip, SourceFillColor, SourceFillRect, SourceGeometryList, SourceGeometryList2, SourceGeometryList3, SourceGeometryList4, SourceGeometryPoint, SourceGeometryRect, SourceGradientFill, SourceGradientStops, SourceIndex, SourcePatternFill } from './part-002.js';
import type { PptxFailure } from './part-003.js';

/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxSourceResponse =
  | {
      index: SourceIndex;
      status: "inspected";
    }
  | {
      error: PptxFailure;
      status: "error";
    };

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;

/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;

export type Digest = string;

export type NativeBlackWhiteMode =
  | "clr"
  | "auto"
  | "gray"
  | "ltGray"
  | "invGray"
  | "grayWhite"
  | "blackGray"
  | "blackWhite"
  | "black"
  | "white"
  | "hidden";

export type SourceBackgroundDefinition =
  | {
      effects?: SourceEffectProperties | null;
      fill: SourceFill;
      kind: "properties";
      retainedOrdinals: number[];
      shadeToTitle?: boolean | null;
      sourceOrdinal: number;
    }
  | {
      color?: SourceColor | null;
      /**
       * 0/1000 mean no fill; 1..999 select fills; 1001+ select background fills.
       * Selection and inherited color substitution belong to resolution.
       */
      index: number;
      kind: "reference";
      retainedOrdinals: number[];
      sourceOrdinal: number;
    };

export type SourceEffectPropertiesDefinition =
  | {
      kind: "list";
      nodes: number[];
    }
  | {
      kind: "dag";
      root: number;
    };

export type SourceFillDefinition =
  | {
      kind: "none";
    }
  | {
      color?: SourceColor | null;
      kind: "solid";
    }
  | {
      flip?: NativeTileFlip | null;
      kind: "gradient";
      rotateWithShape?: boolean | null;
      shade?: SourceGradientShade | null;
      stops?: SourceGradientStops | null;
      tileRect?: SourceFillRect | null;
    }
  | {
      background?: SourceFillColor | null;
      foreground?: SourceFillColor | null;
      kind: "pattern";
      preset?: NativePattern | null;
    }
  | {
      blip?: SourceFillBlip | null;
      dpi?: number | null;
      kind: "image";
      mode?: SourceImageFillMode | null;
      rotateWithShape?: boolean | null;
      sourceRect?: SourceFillRect | null;
    }
  | {
      kind: "group";
    };

export type SourceColorTransform =
  | {
      kind: "tint";
      value: NativePercentage;
    }
  | {
      kind: "shade";
      value: NativePercentage;
    }
  | {
      kind: "comp";
    }
  | {
      kind: "inv";
    }
  | {
      kind: "gray";
    }
  | {
      kind: "alpha";
      value: NativePercentage;
    }
  | {
      kind: "alphaOff";
      value: NativePercentage;
    }
  | {
      kind: "alphaMod";
      value: NativePercentage;
    }
  | {
      kind: "hue";
      value: number;
    }
  | {
      kind: "hueOff";
      value: number;
    }
  | {
      kind: "hueMod";
      value: NativePercentage;
    }
  | {
      kind: "sat";
      value: NativePercentage;
    }
  | {
      kind: "satOff";
      value: NativePercentage;
    }
  | {
      kind: "satMod";
      value: NativePercentage;
    }
  | {
      kind: "lum";
      value: NativePercentage;
    }
  | {
      kind: "lumOff";
      value: NativePercentage;
    }
  | {
      kind: "lumMod";
      value: NativePercentage;
    }
  | {
      kind: "red";
      value: NativePercentage;
    }
  | {
      kind: "redOff";
      value: NativePercentage;
    }
  | {
      kind: "redMod";
      value: NativePercentage;
    }
  | {
      kind: "green";
      value: NativePercentage;
    }
  | {
      kind: "greenOff";
      value: NativePercentage;
    }
  | {
      kind: "greenMod";
      value: NativePercentage;
    }
  | {
      kind: "blue";
      value: NativePercentage;
    }
  | {
      kind: "blueOff";
      value: NativePercentage;
    }
  | {
      kind: "blueMod";
      value: NativePercentage;
    }
  | {
      kind: "gamma";
    }
  | {
      kind: "invGamma";
    };

/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;

export type SourceColorValue =
  | {
      kind: "srgb";
      /**
       * @minItems 3
       * @maxItems 3
       */
      rgb: [number, number, number];
    }
  | {
      blue: NativePercentage;
      green: NativePercentage;
      kind: "scRgb";
      red: NativePercentage;
    }
  | {
      hue: number;
      kind: "hsl";
      luminance: NativePercentage;
      saturation: NativePercentage;
    }
  | {
      color: SystemColor;
      kind: "system";
      /**
       * @minItems 3
       * @maxItems 3
       */
      lastColor?: [number, number, number] | null;
    }
  | {
      kind: "scheme";
      slot: SchemeColor;
    }
  | {
      color: PresetColor;
      kind: "preset";
    };

export type SystemColor =
  | "scrollBar"
  | "background"
  | "activeCaption"
  | "inactiveCaption"
  | "menu"
  | "window"
  | "windowFrame"
  | "menuText"
  | "windowText"
  | "captionText"
  | "activeBorder"
  | "inactiveBorder"
  | "appWorkspace"
  | "highlight"
  | "highlightText"
  | "btnFace"
  | "btnShadow"
  | "grayText"
  | "btnText"
  | "inactiveCaptionText"
  | "btnHighlight"
  | "3dDkShadow"
  | "3dLight"
  | "infoText"
  | "infoBk"
  | "hotLight"
  | "gradientActiveCaption"
  | "gradientInactiveCaption"
  | "menuHighlight"
  | "menuBar";

export type SchemeColor =
  | "bg1"
  | "tx1"
  | "bg2"
  | "tx2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hlink"
  | "folHlink"
  | "phClr"
  | "dk1"
  | "lt1"
  | "dk2"
  | "lt2";

export type PresetColor =
  | "aliceBlue"
  | "antiqueWhite"
  | "aqua"
  | "aquamarine"
  | "azure"
  | "beige"
  | "bisque"
  | "black"
  | "blanchedAlmond"
  | "blue"
  | "blueViolet"
  | "brown"
  | "burlyWood"
  | "cadetBlue"
  | "chartreuse"
  | "chocolate"
  | "coral"
  | "cornflowerBlue"
  | "cornsilk"
  | "crimson"
  | "cyan"
  | "darkBlue"
  | "darkCyan"
  | "darkGoldenrod"
  | "darkGray"
  | "darkGrey"
  | "darkGreen"
  | "darkKhaki"
  | "darkMagenta"
  | "darkOliveGreen"
  | "darkOrange"
  | "darkOrchid"
  | "darkRed"
  | "darkSalmon"
  | "darkSeaGreen"
  | "darkSlateBlue"
  | "darkSlateGray"
  | "darkSlateGrey"
  | "darkTurquoise"
  | "darkViolet"
  | "dkBlue"
  | "dkCyan"
  | "dkGoldenrod"
  | "dkGray"
  | "dkGrey"
  | "dkGreen"
  | "dkKhaki"
  | "dkMagenta"
  | "dkOliveGreen"
  | "dkOrange"
  | "dkOrchid"
  | "dkRed"
  | "dkSalmon"
  | "dkSeaGreen"
  | "dkSlateBlue"
  | "dkSlateGray"
  | "dkSlateGrey"
  | "dkTurquoise"
  | "dkViolet"
  | "deepPink"
  | "deepSkyBlue"
  | "dimGray"
  | "dimGrey"
  | "dodgerBlue"
  | "firebrick"
  | "floralWhite"
  | "forestGreen"
  | "fuchsia"
  | "gainsboro"
  | "ghostWhite"
  | "gold"
  | "goldenrod"
  | "gray"
  | "grey"
  | "green"
  | "greenYellow"
  | "honeydew"
  | "hotPink"
  | "indianRed"
  | "indigo"
  | "ivory"
  | "khaki"
  | "lavender"
  | "lavenderBlush"
  | "lawnGreen"
  | "lemonChiffon"
  | "lightBlue"
  | "lightCoral"
  | "lightCyan"
  | "lightGoldenrodYellow"
  | "lightGray"
  | "lightGrey"
  | "lightGreen"
  | "lightPink"
  | "lightSalmon"
  | "lightSeaGreen"
  | "lightSkyBlue"
  | "lightSlateGray"
  | "lightSlateGrey"
  | "lightSteelBlue"
  | "lightYellow"
  | "ltBlue"
  | "ltCoral"
  | "ltCyan"
  | "ltGoldenrodYellow"
  | "ltGray"
  | "ltGrey"
  | "ltGreen"
  | "ltPink"
  | "ltSalmon"
  | "ltSeaGreen"
  | "ltSkyBlue"
  | "ltSlateGray"
  | "ltSlateGrey"
  | "ltSteelBlue"
  | "ltYellow"
  | "lime"
  | "limeGreen"
  | "linen"
  | "magenta"
  | "maroon"
  | "medAquamarine"
  | "medBlue"
  | "medOrchid"
  | "medPurple"
  | "medSeaGreen"
  | "medSlateBlue"
  | "medSpringGreen"
  | "medTurquoise"
  | "medVioletRed"
  | "mediumAquamarine"
  | "mediumBlue"
  | "mediumOrchid"
  | "mediumPurple"
  | "mediumSeaGreen"
  | "mediumSlateBlue"
  | "mediumSpringGreen"
  | "mediumTurquoise"
  | "mediumVioletRed"
  | "midnightBlue"
  | "mintCream"
  | "mistyRose"
  | "moccasin"
  | "navajoWhite"
  | "navy"
  | "oldLace"
  | "olive"
  | "oliveDrab"
  | "orange"
  | "orangeRed"
  | "orchid"
  | "paleGoldenrod"
  | "paleGreen"
  | "paleTurquoise"
  | "paleVioletRed"
  | "papayaWhip"
  | "peachPuff"
  | "peru"
  | "pink"
  | "plum"
  | "powderBlue"
  | "purple"
  | "red"
  | "rosyBrown"
  | "royalBlue"
  | "saddleBrown"
  | "salmon"
  | "sandyBrown"
  | "seaGreen"
  | "seaShell"
  | "sienna"
  | "silver"
  | "skyBlue"
  | "slateBlue"
  | "slateGray"
  | "slateGrey"
  | "snow"
  | "springGreen"
  | "steelBlue"
  | "tan"
  | "teal"
  | "thistle"
  | "tomato"
  | "turquoise"
  | "violet"
  | "wheat"
  | "white"
  | "whiteSmoke"
  | "yellow"
  | "yellowGreen";

export type NativeTileFlip = "none" | "x" | "y" | "xy";

export type SourceGradientShade =
  | {
      angle?: number | null;
      kind: "linear";
      scaled?: boolean | null;
      sourceOrdinal: number;
    }
  | {
      fillToRect?: SourceFillRect | null;
      kind: "path";
      path?: NativePathShade | null;
      sourceOrdinal: number;
    };

export type NativePathShade = "shape" | "circle" | "rect";

export type NativePattern =
  | "pct5"
  | "pct10"
  | "pct20"
  | "pct25"
  | "pct30"
  | "pct40"
  | "pct50"
  | "pct60"
  | "pct70"
  | "pct75"
  | "pct80"
  | "pct90"
  | "horz"
  | "vert"
  | "ltHorz"
  | "ltVert"
  | "dkHorz"
  | "dkVert"
  | "narHorz"
  | "narVert"
  | "dashHorz"
  | "dashVert"
  | "cross"
  | "dnDiag"
  | "upDiag"
  | "ltDnDiag"
  | "ltUpDiag"
  | "dkDnDiag"
  | "dkUpDiag"
  | "wdDnDiag"
  | "wdUpDiag"
  | "dashDnDiag"
  | "dashUpDiag"
  | "diagCross"
  | "smCheck"
  | "lgCheck"
  | "smGrid"
  | "lgGrid"
  | "dotGrid"
  | "smConfetti"
  | "lgConfetti"
  | "horzBrick"
  | "diagBrick"
  | "solidDmnd"
  | "openDmnd"
  | "dotDmnd"
  | "plaid"
  | "sphere"
  | "weave"
  | "divot"
  | "shingle"
  | "wave"
  | "trellis"
  | "zigZag";

export type NativeBlipCompression = "email" | "screen" | "print" | "hqprint" | "none";

export type SourceImageFillMode =
  | {
      alignment?: NativeFillAlignment | null;
      flip?: NativeTileFlip | null;
      kind: "tile";
      scaleX?: NativePercentage | null;
      scaleY?: NativePercentage | null;
      sourceOrdinal: number;
      translateX?: NativeCoordinate | null;
      translateY?: NativeCoordinate | null;
    }
  | {
      fillRect?: SourceFillRect | null;
      kind: "stretch";
      sourceOrdinal: number;
    };

export type NativeFillAlignment = "tl" | "t" | "tr" | "l" | "ctr" | "r" | "bl" | "b" | "br";

/**
 * Native coordinate: bounded integer EMU or exact decimal universal measure.
 */
export type NativeCoordinate = string;

export type SourceColorMapping =
  | {
      kind: "master";
      sourceOrdinal: number;
    }
  | {
      kind: "explicit";
      mapping: SourceColorMap;
      sourceOrdinal: number;
    };

export type ColorSlot =
  | "dk1"
  | "lt1"
  | "dk2"
  | "lt2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hlink"
  | "folHlink";

export type SourceEffectDefinition =
  | {
      containerType?: NativeEffectContainerType | null;
      kind: "container";
      name?: string | null;
      nodes: number[];
    }
  | {
      kind: "reference";
      reference: string;
    }
  | {
      kind: "alphaBiLevel";
      threshold: NativePercentage;
    }
  | {
      kind: "alphaCeiling";
    }
  | {
      kind: "alphaFloor";
    }
  | {
      color?: SourceColor | null;
      kind: "alphaInverse";
    }
  | {
      container: number;
      kind: "alphaModulate";
    }
  | {
      amount?: NativePercentage | null;
      kind: "alphaModulateFixed";
    }
  | {
      kind: "alphaOutset";
      radius?: NativeCoordinate | null;
    }
  | {
      alpha: NativePercentage;
      kind: "alphaReplace";
    }
  | {
      kind: "biLevel";
      threshold: NativePercentage;
    }
  | {
      blend: NativeBlendMode;
      container: number;
      kind: "blend";
    }
  | {
      grow?: boolean | null;
      kind: "blur";
      radius?: Emu | null;
    }
  | {
      from: SourceColor;
      kind: "colorChange";
      to: SourceColor;
      useAlpha?: boolean | null;
    }
  | {
      color: SourceColor;
      kind: "colorReplace";
    }
  | {
      first: SourceColor;
      kind: "duotone";
      second: SourceColor;
    }
  | {
      fill: SourceFill;
      kind: "fill";
    }
  | {
      blend: NativeBlendMode;
      fill: SourceFill;
      kind: "fillOverlay";
    }
  | {
      color: SourceColor;
      kind: "glow";
      radius?: Emu | null;
    }
  | {
      kind: "grayscale";
    }
  | {
      hue?: number | null;
      kind: "hsl";
      luminance?: NativePercentage | null;
      saturation?: NativePercentage | null;
    }
  | {
      blurRadius?: Emu | null;
      color: SourceColor;
      direction?: number | null;
      distance?: Emu | null;
      kind: "innerShadow";
    }
  | {
      brightness?: NativePercentage | null;
      contrast?: NativePercentage | null;
      kind: "luminance";
    }
  | {
      alignment?: NativeFillAlignment | null;
      blurRadius?: Emu | null;
      color: SourceColor;
      direction?: number | null;
      distance?: Emu | null;
      kind: "outerShadow";
      rotateWithShape?: boolean | null;
      scaleX?: NativePercentage | null;
      scaleY?: NativePercentage | null;
      skewX?: number | null;
      skewY?: number | null;
    }
  | {
      color: SourceColor;
      direction?: number | null;
      distance?: Emu | null;
      kind: "presetShadow";
      preset: NativePresetShadow;
    }
  | {
      alignment?: NativeFillAlignment | null;
      blurRadius?: Emu | null;
      direction?: number | null;
      distance?: Emu | null;
      endAlpha?: NativePercentage | null;
      endPosition?: NativePercentage | null;
      fadeDirection?: number | null;
      kind: "reflection";
      rotateWithShape?: boolean | null;
      scaleX?: NativePercentage | null;
      scaleY?: NativePercentage | null;
      skewX?: number | null;
      skewY?: number | null;
      startAlpha?: NativePercentage | null;
      startPosition?: NativePercentage | null;
    }
  | {
      kind: "relativeOffset";
      translateX?: NativePercentage | null;
      translateY?: NativePercentage | null;
    }
  | {
      kind: "softEdge";
      radius: Emu;
    }
  | {
      amount?: NativePercentage | null;
      hue?: number | null;
      kind: "tint";
    }
  | {
      kind: "transform";
      scaleX?: NativePercentage | null;
      scaleY?: NativePercentage | null;
      skewX?: number | null;
      skewY?: number | null;
      translateX?: NativeCoordinate | null;
      translateY?: NativeCoordinate | null;
    };

export type NativeEffectContainerType = "sib" | "tree";

export type NativeBlendMode = "over" | "mult" | "screen" | "darken" | "lighten";

export type NativePresetShadow =
  | "shdw1"
  | "shdw2"
  | "shdw3"
  | "shdw4"
  | "shdw5"
  | "shdw6"
  | "shdw7"
  | "shdw8"
  | "shdw9"
  | "shdw10"
  | "shdw11"
  | "shdw12"
  | "shdw13"
  | "shdw14"
  | "shdw15"
  | "shdw16"
  | "shdw17"
  | "shdw18"
  | "shdw19"
  | "shdw20";

export type SurfaceKind = "slide" | "master" | "layout";

export type SourceGeometryDefinition =
  | {
      adjustments?: SourceGeometryList | null;
      kind: "preset";
      preset: NativeShapeType;
    }
  | {
      adjustments?: SourceGeometryList | null;
      connections?: SourceGeometryList3 | null;
      guides?: SourceGeometryList | null;
      handles?: SourceGeometryList2 | null;
      kind: "custom";
      paths: SourceGeometryList4;
      textRect?: SourceGeometryRect | null;
    };

export type NativeShapeType =
  | "accentBorderCallout1"
  | "accentBorderCallout2"
  | "accentBorderCallout3"
  | "accentCallout1"
  | "accentCallout2"
  | "accentCallout3"
  | "actionButtonBackPrevious"
  | "actionButtonBeginning"
  | "actionButtonBlank"
  | "actionButtonDocument"
  | "actionButtonEnd"
  | "actionButtonForwardNext"
  | "actionButtonHelp"
  | "actionButtonHome"
  | "actionButtonInformation"
  | "actionButtonMovie"
  | "actionButtonReturn"
  | "actionButtonSound"
  | "arc"
  | "bentArrow"
  | "bentConnector2"
  | "bentConnector3"
  | "bentConnector4"
  | "bentConnector5"
  | "bentUpArrow"
  | "bevel"
  | "blockArc"
  | "borderCallout1"
  | "borderCallout2"
  | "borderCallout3"
  | "bracePair"
  | "bracketPair"
  | "callout1"
  | "callout2"
  | "callout3"
  | "can"
  | "chartPlus"
  | "chartStar"
  | "chartX"
  | "chevron"
  | "chord"
  | "circularArrow"
  | "cloud"
  | "cloudCallout"
  | "corner"
  | "cornerTabs"
  | "cube"
  | "curvedConnector2"
  | "curvedConnector3"
  | "curvedConnector4"
  | "curvedConnector5"
  | "curvedDownArrow"
  | "curvedLeftArrow"
  | "curvedRightArrow"
  | "curvedUpArrow"
  | "decagon"
  | "diagStripe"
  | "diamond"
  | "dodecagon"
  | "donut"
  | "doubleWave"
  | "downArrow"
  | "downArrowCallout"
  | "ellipse"
  | "ellipseRibbon"
  | "ellipseRibbon2"
  | "flowChartAlternateProcess"
  | "flowChartCollate"
  | "flowChartConnector"
  | "flowChartDecision"
  | "flowChartDelay"
  | "flowChartDisplay"
  | "flowChartDocument"
  | "flowChartExtract"
  | "flowChartInputOutput"
  | "flowChartInternalStorage"
  | "flowChartMagneticDisk"
  | "flowChartMagneticDrum"
  | "flowChartMagneticTape"
  | "flowChartManualInput"
  | "flowChartManualOperation"
  | "flowChartMerge"
  | "flowChartMultidocument"
  | "flowChartOfflineStorage"
  | "flowChartOffpageConnector"
  | "flowChartOnlineStorage"
  | "flowChartOr"
  | "flowChartPredefinedProcess"
  | "flowChartPreparation"
  | "flowChartProcess"
  | "flowChartPunchedCard"
  | "flowChartPunchedTape"
  | "flowChartSort"
  | "flowChartSummingJunction"
  | "flowChartTerminator"
  | "foldedCorner"
  | "frame"
  | "funnel"
  | "gear6"
  | "gear9"
  | "halfFrame"
  | "heart"
  | "heptagon"
  | "hexagon"
  | "homePlate"
  | "horizontalScroll"
  | "irregularSeal1"
  | "irregularSeal2"
  | "leftArrow"
  | "leftArrowCallout"
  | "leftBrace"
  | "leftBracket"
  | "leftCircularArrow"
  | "leftRightArrow"
  | "leftRightArrowCallout"
  | "leftRightCircularArrow"
  | "leftRightRibbon"
  | "leftRightUpArrow"
  | "leftUpArrow"
  | "lightningBolt"
  | "line"
  | "lineInv"
  | "mathDivide"
  | "mathEqual"
  | "mathMinus"
  | "mathMultiply"
  | "mathNotEqual"
  | "mathPlus"
  | "moon"
  | "noSmoking"
  | "nonIsoscelesTrapezoid"
  | "notchedRightArrow"
  | "octagon"
  | "parallelogram"
  | "pentagon"
  | "pie"
  | "pieWedge"
  | "plaque"
  | "plaqueTabs"
  | "plus"
  | "quadArrow"
  | "quadArrowCallout"
  | "rect"
  | "ribbon"
  | "ribbon2"
  | "rightArrow"
  | "rightArrowCallout"
  | "rightBrace"
  | "rightBracket"
  | "round1Rect"
  | "round2DiagRect"
  | "round2SameRect"
  | "roundRect"
  | "rtTriangle"
  | "smileyFace"
  | "snip1Rect"
  | "snip2DiagRect"
  | "snip2SameRect"
  | "snipRoundRect"
  | "squareTabs"
  | "star10"
  | "star12"
  | "star16"
  | "star24"
  | "star32"
  | "star4"
  | "star5"
  | "star6"
  | "star7"
  | "star8"
  | "straightConnector1"
  | "stripedRightArrow"
  | "sun"
  | "swooshArrow"
  | "teardrop"
  | "trapezoid"
  | "triangle"
  | "upArrow"
  | "upArrowCallout"
  | "upDownArrow"
  | "upDownArrowCallout"
  | "uturnArrow"
  | "verticalScroll"
  | "wave"
  | "wedgeEllipseCallout"
  | "wedgeRectCallout"
  | "wedgeRoundRectCallout";

export type SourceAdjustHandle =
  | {
      guideX?: string | null;
      guideY?: string | null;
      kind: "xy";
      maxX?: string | null;
      maxY?: string | null;
      minX?: string | null;
      minY?: string | null;
      position: SourceGeometryPoint;
      sourceOrdinal: number;
    }
  | {
      guideAngle?: string | null;
      guideRadius?: string | null;
      kind: "polar";
      maxAngle?: string | null;
      maxRadius?: string | null;
      minAngle?: string | null;
      minRadius?: string | null;
      position: SourceGeometryPoint;
      sourceOrdinal: number;
    };

export type SourceGeometryCommand =
  | {
      kind: "move";
      sourceOrdinal: number;
      to: SourceGeometryPoint;
    }
  | {
      kind: "line";
      sourceOrdinal: number;
      to: SourceGeometryPoint;
    }
  | {
      heightRadius: string;
      kind: "arc";
      sourceOrdinal: number;
      startAngle: string;
      sweepAngle: string;
      widthRadius: string;
    }
  | {
      control: SourceGeometryPoint;
      kind: "quadratic";
      sourceOrdinal: number;
      to: SourceGeometryPoint;
    }
  | {
      control1: SourceGeometryPoint;
      control2: SourceGeometryPoint;
      kind: "cubic";
      sourceOrdinal: number;
      to: SourceGeometryPoint;
    }
  | {
      kind: "close";
      sourceOrdinal: number;
    };

export type NativePathFill = "none" | "norm" | "lighten" | "lightenLess" | "darken" | "darkenLess";

export type SourceObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";

export type NativePenAlignment = "ctr" | "in";

export type NativeLineCap = "flat" | "rnd" | "sq";

export type NativeCompoundLine = "sng" | "dbl" | "thickThin" | "thinThick" | "tri";

export type SourceLineDash =
  | {
      kind: "preset";
      sourceOrdinal: number;
      value?: NativePresetDash | null;
    }
  | {
      kind: "custom";
      sourceOrdinal: number;
      stops: SourceDashStop[];
    };

export type NativePresetDash =
  | "solid"
  | "dot"
  | "dash"
  | "lgDash"
  | "dashDot"
  | "lgDashDot"
  | "lgDashDotDot"
  | "sysDash"
  | "sysDot"
  | "sysDashDot"
  | "sysDashDotDot";

export type SourceLineFill =
  | {
      kind: "none";
      sourceOrdinal: number;
    }
  | {
      color?: SourceColor | null;
      kind: "solid";
      sourceOrdinal: number;
    }
  | {
      gradient: SourceGradientFill;
      kind: "gradient";
      sourceOrdinal: number;
    }
  | {
      kind: "pattern";
      pattern: SourcePatternFill;
      sourceOrdinal: number;
    };

export type NativeLineEnd = "none" | "triangle" | "stealth" | "diamond" | "oval" | "arrow";

export type NativeLineEndSize = "sm" | "med" | "lg";

export type SourceLineJoin =
  | {
      kind: "round";
      sourceOrdinal: number;
    }
  | {
      kind: "bevel";
      sourceOrdinal: number;
    }
  | {
      kind: "miter";
      limit?: NativePercentage | null;
      sourceOrdinal: number;
    };

export type SourceTextConstraint = "compatibilityBranch" | "structuredLeaf" | "dynamicField" | "timingReferences";

export type SourceRunKind = "text" | "break" | "field";

export type PlaceholderKind =
  | "title"
  | "body"
  | "ctrTitle"
  | "subTitle"
  | "dt"
  | "sldNum"
  | "ftr"
  | "hdr"
  | "obj"
  | "chart"
  | "tbl"
  | "clipArt"
  | "dgm"
  | "media"
  | "sldImg"
  | "pic";

export type PlaceholderOrientation = "horz" | "vert";

export type PlaceholderSize = "full" | "half" | "quarter";
