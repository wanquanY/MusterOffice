/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { EffectiveFillRect, EffectiveFillTile, EffectiveGradientFill, EffectiveImageFill, EffectiveLine, EffectiveLineGeometry, EffectivePatternFill, FillColorEvaluation, FillColorExpression, FillOwner, FillRedirect, FillValue3, FillValue4, FillValue5, LineColorExpression, LineValue5, LineValue6, NativeShapeType, SourceCellAddress, SourceDashStop, SourceObjectRef, SourcePagePlan } from './part-002.js';
import type { PptxPageFailure } from './part-003.js';

/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxPageCompileResponse =
  | {
      plan: SourcePagePlan;
      status: "compiled";
    }
  | {
      error: PptxPageFailure;
      status: "error";
    };

export type FillPaintColors =
  | {
      kind: "none";
    }
  | {
      kind: "unresolvedStyle";
    }
  | {
      kind: "imageResourcesRequired";
    }
  | {
      color: FillColorEvaluation;
      kind: "solid";
    }
  | {
      kind: "gradient";
      stops: FillColorEvaluation[];
    }
  | {
      background: FillColorEvaluation;
      foreground: FillColorEvaluation;
      kind: "pattern";
    };

export type ColorDependency =
  | {
      kind: "theme";
      part: string;
      slot: ColorSlot;
      sourceOrdinal: number;
    }
  | {
      color: SystemColor;
      kind: "system";
      origin: SystemColorOrigin;
    }
  | {
      kind: "placeholder";
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

export type SystemColorOrigin = "hostContext" | "fileLastColor";

export type ColorNotice = "grayWeightsProvisional" | "presetAliasDiscrepancy";

export type ColorSample =
  | {
      clippedForSrgb: boolean;
      /**
       * @minItems 4
       * @maxItems 4
       */
      linear: [number, number, number, number];
      /**
       * @minItems 4
       * @maxItems 4
       */
      rgba16: [number, number, number, number];
      /**
       * Never feed these quantized samples back into native expressions.
       *
       * @minItems 4
       * @maxItems 4
       */
      rgba8: [number, number, number, number];
      /**
       * Unassociated, unclipped working-precision channels.
       *
       * @minItems 4
       * @maxItems 4
       */
      srgb: [number, number, number, number];
      status: "resolved";
    }
  | {
      reason: ColorUnresolved;
      status: "unresolved";
    };

export type ColorUnresolved =
  | {
      kind: "missingColorMap";
    }
  | {
      kind: "missingColorScheme";
    }
  | {
      kind: "missingThemeSlot";
      slot: ColorSlot;
    }
  | {
      color: SystemColor;
      kind: "missingSystemColor";
    }
  | {
      kind: "missingPlaceholder";
    }
  | {
      kind: "retainedPlaceholderContext";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "schemeCycle";
      slot: ColorSlot;
    }
  | {
      kind: "numericRange";
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
      kind: "chart";
      part: string;
      sourceOrdinal: number;
    }
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

export type LinePaintColor =
  | {
      kind: "none";
    }
  | {
      kind: "unresolvedStyle";
    }
  | {
      dependencies: ColorDependency[];
      kind: "solid";
      notices: ColorNotice[];
      outcome: ColorSample;
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

export type NativeRetainedLineFill = "gradient" | "pattern";

/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;

export type TransformValueSource =
  | {
      kind: "declaration";
      object: SourceObjectRef;
    }
  | {
      kind: "default";
      object: SourceObjectRef;
    };

export type LineGeometryOutcome =
  | {
      geometry: EffectiveLineGeometry;
      status: "resolved";
    }
  | {
      reason: LineUnresolved;
      status: "unresolved";
    };

export type SurfaceKind = "slide" | "master" | "layout";

export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";

export type Digest = string;

export type PagePaintKind = "fill" | "stroke";

/**
 * Ordinals are zero-based element preorder in the indicated XML resource.
 * Preset ordinals address the pinned, generated catalog definition, not the
 * document part. A document override always retains its physical source.
 */
export type GeometryOrigin =
  | {
      kind: "document";
      sourceOrdinal: number;
    }
  | {
      definitionOrdinal: number;
      kind: "preset";
      preset: NativeShapeType;
    };
