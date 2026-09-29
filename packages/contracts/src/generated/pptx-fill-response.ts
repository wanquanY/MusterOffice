/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxFillResponse =
  | {
      status: "evaluated";
      styles: SourceFillStyles;
    }
  | {
      error: PptxFailure;
      status: "error";
    };
/**
 * An explicit interpretation, not a certificate for any Office/WPS version.
 */
export type FillProfile = "ms-oi29500-fills-2024-draft-v1";
export type Digest = string;
export type FillOutcome =
  | {
      fill: EffectiveFill;
      redirects: FillRedirect[];
      status: "resolved";
    }
  | {
      reason: FillUnresolved;
      status: "unresolved";
    };
export type EffectiveFill =
  | {
      declaredBy: FillOrigin;
      kind: "none";
    }
  | {
      color: FillColorExpression;
      declaredBy: FillOrigin;
      kind: "solid";
    }
  | {
      declaredBy: FillOrigin;
      gradient: EffectiveGradientFill;
      kind: "gradient";
    }
  | {
      declaredBy: FillOrigin;
      kind: "pattern";
      pattern: EffectivePatternFill;
    }
  | {
      declaredBy: FillOrigin;
      image: EffectiveImageFill;
      kind: "image";
    };
export type FillOrigin =
  | {
      kind: "declaration";
      owner: FillOwner;
      sourceOrdinal: number;
    }
  | {
      kind: "tableStyle";
      part: string;
      sourceOrdinal: number;
      via: FillOwner;
    }
  | {
      kind: "theme";
      part: string;
      referenceOrdinal: number;
      sourceOrdinal: number;
      styleIndex: number;
      via: FillOwner;
    }
  | {
      kind: "schemaDefault";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "profileDefault";
    };
export type FillTarget =
  | {
      kind: "object";
      nativeId: number;
    }
  | {
      kind: "line";
      nativeId: number;
    }
  | {
      kind: "picture";
      nativeId: number;
    }
  | {
      cell: SourceCellAddress;
      kind: "tableCell";
      nativeId: number;
    }
  | {
      cell: SourceCellAddress;
      edge: TableCellEdge;
      kind: "tableCellBorder";
      nativeId: number;
    }
  | {
      kind: "tableBackground";
      nativeId: number;
    }
  | {
      kind: "tableStyleFill";
      nativeId: number;
      region?: TableStyleRegion | null;
    }
  | {
      edge: TableStyleEdge;
      kind: "tableStyleBorder";
      nativeId: number;
      region: TableStyleRegion;
    }
  | {
      kind: "rootGroup";
    }
  | {
      kind: "background";
    };
export type TableCellEdge = "left" | "right" | "top" | "bottom" | "topLeftToBottomRight" | "bottomLeftToTopRight";
export type TableStyleRegion =
  | "wholeTbl"
  | "band1H"
  | "band2H"
  | "band1V"
  | "band2V"
  | "lastCol"
  | "firstCol"
  | "lastRow"
  | "seCell"
  | "swCell"
  | "firstRow"
  | "neCell"
  | "nwCell";
export type TableStyleEdge =
  | "left"
  | "right"
  | "top"
  | "bottom"
  | "insideHorizontal"
  | "insideVertical"
  | "topLeftToBottomRight"
  | "topRightToBottomLeft";
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
export type EffectiveGradientShade =
  | {
      angle: FillValue3;
      declaredBy: FillOrigin;
      kind: "linear";
      scaled: FillValue4;
    }
  | {
      declaredBy: FillOrigin;
      fillToRect: EffectiveFillRect;
      kind: "path";
      path: FillValue5;
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
export type EffectiveImageMode =
  | {
      declaredBy: FillOrigin;
      kind: "tile";
      tile: EffectiveFillTile;
    }
  | {
      declaredBy: FillOrigin;
      fillRect: EffectiveFillRect;
      kind: "stretch";
    };
export type NativeFillAlignment = "tl" | "t" | "tr" | "l" | "ctr" | "r" | "bl" | "b" | "br";
/**
 * Native coordinate: bounded integer EMU or exact decimal universal measure.
 */
export type NativeCoordinate = string;
export type FillUnresolved =
  | {
      kind: "tableGrid";
      owner: FillOwner;
      reason: NativeTableGridIssue;
    }
  | {
      kind: "tableStyle";
      owner: FillOwner;
      reason: TableStyleSelectionError;
    }
  | {
      kind: "unsupportedTarget";
      owner: FillOwner;
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
      owner: FillOwner;
    }
  | {
      kind: "missingFormatScheme";
      owner: FillOwner;
    }
  | {
      available: number;
      index: number;
      kind: "styleIndexOutOfRange";
      owner: FillOwner;
    }
  | {
      kind: "retainedContent";
      origin: FillOrigin;
    }
  | {
      kind: "effectEvaluationRequired";
      origin: FillOrigin;
    }
  | {
      kind: "missingImage";
      origin: FillOrigin;
    }
  | {
      kind: "groupWithoutParent";
      origin: FillOrigin;
    }
  | {
      kind: "unsupportedBackgroundMode";
      owner: FillOwner;
    };
export type NativeTableGridIssue =
  | {
      kind: "emptyGrid";
    }
  | {
      actual: number;
      expected: number;
      kind: "rowWidth";
      row: number;
    }
  | {
      cell: SourceCellAddress;
      kind: "duplicateCellId";
    }
  | {
      cell: SourceCellAddress;
      kind: "invalidSpan";
    }
  | {
      cell: SourceCellAddress;
      kind: "missingNeighbour";
    }
  | {
      cell: SourceCellAddress;
      kind: "conflictingNeighbours";
    }
  | {
      cell: SourceCellAddress;
      kind: "outsideMerge";
      origin: SourceCellAddress;
    }
  | {
      cell: SourceCellAddress;
      kind: "conflictingSpan";
      origin: SourceCellAddress;
    }
  | {
      kind: "incompleteMerge";
      origin: SourceCellAddress;
    };
export type TableStyleSelectionError =
  | {
      kind: "conflictingStyles";
    }
  | {
      kind: "invalidIdentity";
    }
  | {
      kind: "missingDefinition";
    }
  | {
      kind: "gridMismatch";
    }
  | {
      kind: "cellOutsideGrid";
    }
  | {
      kind: "retainedDeclaration";
      sourceOrdinal: number;
    }
  | {
      kind: "cancelled";
    };
export type SourcePlaceholderMatch =
  | {
      status: "notPlaceholder";
    }
  | {
      status: "master";
    }
  | {
      rule: PlaceholderMatchRule;
      status: "matched";
      target: SourceObjectRef;
    }
  | {
      status: "unmatched";
    }
  | {
      status: "detached";
    }
  | {
      candidates: number;
      part: string;
      status: "ambiguous";
    }
  | {
      status: "unsupportedContext";
    };
export type PlaceholderMatchRule = "slideIndex" | "masterType";
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceFillStyles {
  profile: FillProfile;
  sourceSha256: Digest;
  surface: string;
  targets: SourceFillResult[];
}
export interface SourceFillResult {
  outcome: FillOutcome;
  target: FillTarget;
}
export interface FillOwner {
  part: string;
  target: FillTarget;
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
export interface FillColorExpression {
  color: FillColorTerm;
  /**
   * Resolve a style reference color in this native owner only if color
   * evaluation actually reaches phClr. Inheritance does not sample colors.
   */
  contextOwner?: FillOwner | null;
}
export interface FillColorTerm {
  declaredBy: FillOrigin;
  transforms: SourceColorTransform[];
  value: SourceColorValue;
}
export interface EffectiveGradientFill {
  flip: FillValue6;
  rotateWithShape: FillValue4;
  shade: EffectiveGradientShade;
  stops: FillValue;
  tileRect: EffectiveFillRect;
}
export interface FillValue6 {
  declaredBy: FillOrigin;
  value: NativeTileFlip;
}
export interface FillValue4 {
  declaredBy: FillOrigin;
  value: boolean;
}
export interface FillValue3 {
  declaredBy: FillOrigin;
  value: number;
}
export interface EffectiveFillRect {
  bottom: FillValue2;
  declaredBy: FillOrigin;
  left: FillValue2;
  right: FillValue2;
  top: FillValue2;
}
export interface FillValue2 {
  declaredBy: FillOrigin;
  value: NativePercentage;
}
export interface FillValue5 {
  declaredBy: FillOrigin;
  value: NativePathShade;
}
export interface FillValue {
  declaredBy: FillOrigin;
  value: EffectiveGradientStop[];
}
export interface EffectiveGradientStop {
  color: FillColorExpression;
  position: FillValue2;
}
export interface EffectivePatternFill {
  background: FillColorExpression;
  foreground: FillColorExpression;
  preset: FillValue7;
}
export interface FillValue7 {
  declaredBy: FillOrigin;
  value: NativePattern;
}
export interface EffectiveImageFill {
  compression: FillValue9;
  dpi: FillValue3;
  embed: FillValue8;
  link: FillValue81;
  mode: EffectiveImageMode;
  rotateWithShape: FillValue4;
  sourceRect: EffectiveFillRect;
}
export interface FillValue9 {
  declaredBy: FillOrigin;
  value: NativeBlipCompression;
}
/**
 * Each relationship belongs to the part in its own declaring origin.
 * Empty strings are explicit or profile-default empty relationship IDs.
 */
export interface FillValue8 {
  declaredBy: FillOrigin;
  value: string;
}
export interface FillValue81 {
  declaredBy: FillOrigin;
  value: string;
}
export interface EffectiveFillTile {
  alignment: FillValue11;
  flip: FillValue6;
  scaleX: FillValue2;
  scaleY: FillValue2;
  translateX: FillValue10;
  translateY: FillValue10;
}
export interface FillValue11 {
  declaredBy: FillOrigin;
  value: NativeFillAlignment;
}
export interface FillValue10 {
  declaredBy: FillOrigin;
  value: NativeCoordinate;
}
export interface FillRedirect {
  declaredBy: FillOrigin;
  target: FillOwner;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
