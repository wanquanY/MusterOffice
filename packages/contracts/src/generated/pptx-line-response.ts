/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxLineResponse =
  | {
      status: "evaluated";
      styles: SourceLineStyles;
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type LineOutcome =
  | {
      line: EffectiveLine;
      status: "resolved";
    }
  | {
      reason: LineUnresolved;
      status: "unresolved";
    };
export type LineOrigin =
  | {
      kind: "chart";
      part: string;
      sourceOrdinal: number;
    }
  | {
      cell: SourceCellAddress;
      edge: TableCellEdge;
      kind: "tableCell";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      edge: TableStyleEdge;
      kind: "tableStyle";
      object: SourceObjectRef;
      part: string;
      region: TableStyleRegion;
      sourceOrdinal: number;
    }
  | {
      edge: TableStyleEdge;
      kind: "tableTheme";
      part: string;
      referenceOrdinal: number;
      region: TableStyleRegion;
      sourceOrdinal: number;
      styleIndex: number;
      via: SourceObjectRef;
    }
  | {
      kind: "object";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      kind: "theme";
      part: string;
      referenceOrdinal: number;
      sourceOrdinal: number;
      styleIndex: number;
      via: SourceObjectRef;
    }
  | {
      kind: "profileDefault";
    };
export type TableCellEdge = "left" | "right" | "top" | "bottom" | "topLeftToBottomRight" | "bottomLeftToTopRight";
export type TableStyleEdge =
  | "left"
  | "right"
  | "top"
  | "bottom"
  | "insideHorizontal"
  | "insideVertical"
  | "topLeftToBottomRight"
  | "topRightToBottomLeft";
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
export type NativePenAlignment = "ctr" | "in";
export type NativeLineCap = "flat" | "rnd" | "sq";
export type NativeCompoundLine = "sng" | "dbl" | "thickThin" | "thinThick" | "tri";
export type EffectiveLineDash =
  | {
      declaredBy: LineOrigin;
      kind: "preset";
      value: LineValue5;
    }
  | {
      declaredBy: LineOrigin;
      kind: "custom";
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
/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;
export type EffectiveLineFill =
  | {
      declaredBy: LineOrigin;
      kind: "none";
    }
  | {
      color: LineColorExpression;
      declaredBy: LineOrigin;
      kind: "solid";
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
export type NativeLineEnd = "none" | "triangle" | "stealth" | "diamond" | "oval" | "arrow";
export type NativeLineEndSize = "sm" | "med" | "lg";
export type EffectiveLineJoin =
  | {
      declaredBy: LineOrigin;
      kind: "round";
    }
  | {
      declaredBy: LineOrigin;
      kind: "bevel";
    }
  | {
      declaredBy: LineOrigin;
      kind: "miter";
      limit: LineValue6;
    };
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type LineUnresolved =
  | {
      kind: "tableGrid";
      object: SourceObjectRef;
      reason: NativeTableGridIssue;
    }
  | {
      kind: "tableStyle";
      object: SourceObjectRef;
      reason: TableStyleSelectionError;
    }
  | {
      kind: "unsupportedObject";
      object: SourceObjectRef;
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
      object: SourceObjectRef;
    }
  | {
      kind: "missingFormatScheme";
      object: SourceObjectRef;
    }
  | {
      available: number;
      index: number;
      kind: "styleIndexOutOfRange";
      object: SourceObjectRef;
    }
  | {
      kind: "retainedContent";
      origin: LineOrigin;
    }
  | {
      kind: "unsupportedFill";
      nativeKind: NativeRetainedLineFill;
      origin: LineOrigin;
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
export type NativeRetainedLineFill = "gradient" | "pattern";
/**
 * Explicit interpretation of the documented multi-pass DrawingML rules.
 * This is not a certificate for a particular Office/WPS version or renderer.
 */
export type LineProfile = "ms-oi29500-lines-2024-draft-v1";
export type Digest = string;
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceLineStyles {
  /**
   * Request order is retained, including duplicates.
   */
  objects: SourceLineResult[];
  profile: LineProfile;
  sourceSha256: Digest;
  surface: string;
}
export interface SourceLineResult {
  nativeId: number;
  outcome: LineOutcome;
}
export interface EffectiveLine {
  alignment: LineValue4;
  cap: LineValue2;
  compound: LineValue3;
  dash: EffectiveLineDash;
  fill: EffectiveLineFill;
  head: EffectiveLineEnd;
  join: EffectiveLineJoin;
  tail: EffectiveLineEnd;
  width: LineValue;
}
export interface LineValue4 {
  declaredBy: LineOrigin;
  value: NativePenAlignment;
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface LineValue2 {
  declaredBy: LineOrigin;
  value: NativeLineCap;
}
export interface LineValue3 {
  declaredBy: LineOrigin;
  value: NativeCompoundLine;
}
export interface LineValue5 {
  declaredBy: LineOrigin;
  value: NativePresetDash;
}
export interface SourceDashStop {
  dash: NativePercentage;
  sourceOrdinal: number;
  space: NativePercentage;
}
/**
 * Retains native color operations and their owning style context. Colors are
 * not sampled/quantized during line inheritance; color evaluation is separate.
 */
export interface LineColorExpression {
  color: LineColorTerm;
  placeholder?: LineColorTerm | null;
}
export interface LineColorTerm {
  declaredBy: LineOrigin;
  transforms: SourceColorTransform[];
  value: SourceColorValue;
}
export interface EffectiveLineEnd {
  declaredBy: LineOrigin;
  kind: LineValue7;
  length: LineValue8;
  width: LineValue8;
}
export interface LineValue7 {
  declaredBy: LineOrigin;
  value: NativeLineEnd;
}
export interface LineValue8 {
  declaredBy: LineOrigin;
  value: NativeLineEndSize;
}
export interface LineValue6 {
  declaredBy: LineOrigin;
  value: NativePercentage;
}
export interface LineValue {
  declaredBy: LineOrigin;
  value: Emu;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
