/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxChartPaintResponse =
  | {
      paints: SourceChartPaints;
      status: "computed";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type ChartAxisKind = "category" | "value" | "date" | "series";
export type ChartMarkupKind =
  | "shapeProperties"
  | "textProperties"
  | "title"
  | "majorGridlines"
  | "minorGridlines"
  | "displayUnits"
  | "dataLabels"
  | "seriesLines"
  | "marker"
  | "pictureOptions"
  | "trendline"
  | "errorBars"
  | "extensions";
export type ChartPropertyKind =
  | "barDirection"
  | "grouping"
  | "varyColors"
  | "gapWidth"
  | "gapDepth"
  | "overlap"
  | "firstSliceAngle"
  | "holeSize"
  | "delete"
  | "axisPosition"
  | "majorTickMark"
  | "minorTickMark"
  | "tickLabelPosition"
  | "crossAxis"
  | "crosses"
  | "crossesAt"
  | "crossBetween"
  | "majorUnit"
  | "minorUnit"
  | "auto"
  | "labelAlignment"
  | "labelOffset"
  | "tickLabelSkip"
  | "tickMarkSkip"
  | "noMultiLevelLabels"
  | "baseTimeUnit"
  | "majorTimeUnit"
  | "minorTimeUnit"
  | "logBase"
  | "orientation"
  | "minimum"
  | "maximum"
  | "explosion"
  | "bubble3D"
  | "invertIfNegative"
  | "smooth";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type ChartWorkbookTarget =
  | {
      byteLength: ByteLength;
      contentType: string;
      kind: "embedded";
      part: string;
      sha256: Digest;
    }
  | {
      kind: "external";
      uri: string;
    };
export type Digest = string;
export type ChartCacheKind = "number" | "string" | "multiLevelString";
export type ChartChannelRole = "title" | "categories" | "values" | "xValues" | "yValues" | "bubbleSize";
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
/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;
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
/**
 * Native coordinate: bounded integer EMU or exact decimal universal measure.
 */
export type NativeCoordinate = string;
export type NativeBlendMode = "over" | "mult" | "screen" | "darken" | "lighten";
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
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
export type SourceEffectPropertiesDefinition =
  | {
      kind: "list";
      nodes: number[];
    }
  | {
      kind: "dag";
      root: number;
    };
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
/**
 * A named, provisional numerical interpretation, not an Office/WPS certificate.
 */
export type ColorProfile = "ecma376-2016-draft-v1";
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceChartPaints {
  chart: SourceChartPart;
  colorMapping?: SourceColorMapRef | null;
  colorScheme?: SourceThemeSchemeRef | null;
  /**
   * Source order. This is not a resolved chart-style/series/point cascade.
   */
  declarations: ChartPaintDeclaration[];
  object: SourceObjectRef;
  profile: ColorProfile;
  sourceSha256: Digest;
  themeOverride?: ChartThemeOverride | null;
}
/**
 * Series and point layout references bind to the declaration ordinals below.
 */
export interface SourceChartPart {
  /**
   * Source declarations; reference resolution, defaults and geometry are separate.
   */
  axes: SourceChartAxis[];
  byteLength: ByteLength;
  compatibility: SourceCompatibility;
  /**
   * Cache declarations are snapshots, not recalculated workbook values.
   */
  dataAuthority: "sourceCacheSnapshot";
  /**
   * Opaque extension roots remain physically bound even though inspection
   * does not interpret their descendants. Consumers must not silently admit.
   */
  extensionOrdinals?: number[];
  externalData?: SourceChartExternalData | null;
  part: string;
  plots: SourceChartPlot[];
  sha256: Digest;
}
export interface SourceChartAxis {
  id: number;
  kind: ChartAxisKind;
  layout: SourceChartLayout;
  numberFormat?: SourceChartNumberFormat | null;
  scaling?: SourceChartScaling | null;
  sourceOrdinal: number;
}
export interface SourceChartLayout {
  /**
   * Complex markup remains bound to its original part and physical ordinal.
   */
  markup: SourceChartMarkup[];
  /**
   * Source order; missing property, missing val and explicit lexical val differ.
   */
  properties: SourceChartProperty[];
  unrecognizedChildren?: SourceChartUnknown[];
}
export interface SourceChartMarkup {
  kind: ChartMarkupKind;
  sourceOrdinal: number;
}
export interface SourceChartProperty {
  kind: ChartPropertyKind;
  sourceOrdinal: number;
  /**
   * XML attribute value, without numeric conversion or schema defaulting.
   */
  value?: string | null;
}
export interface SourceChartUnknown {
  localName: string;
  namespace: string;
  sourceOrdinal: number;
}
export interface SourceChartNumberFormat {
  formatCode?: string | null;
  /**
   * Preserve omission and the actual lexical boolean; no workbook lookup.
   */
  sourceLinked?: string | null;
  sourceOrdinal: number;
}
export interface SourceChartScaling {
  properties: SourceChartProperty[];
  sourceOrdinal: number;
}
export interface SourceCompatibility {
  ignoredAttributes: number;
  ignoredElements: number;
  selections: SourceCompatibilitySelection[];
  unwrappedElements: number;
}
export interface SourceCompatibilitySelection {
  branches: SourceCompatibilityBranch[];
  sourceOrdinal: number;
}
export interface SourceCompatibilityBranch {
  fallback: boolean;
  requires: string[];
  selected: boolean;
  sourceOrdinal: number;
}
export interface SourceChartExternalData {
  /**
   * None means omitted; no eager refresh is performed by inspection.
   */
  autoUpdate?: boolean | null;
  relationshipId: string;
  sourceOrdinal: number;
  target: ChartWorkbookTarget;
}
export interface SourceChartPlot {
  axisIds: number[];
  layout: SourceChartLayout;
  /**
   * Native local name (barChart, doughnutChart, ...), not a rendering capability.
   */
  nativeKind: string;
  series: SourceChartSeries[];
  sourceOrdinal: number;
}
export interface SourceChartSeries {
  channels: SourceChartChannel[];
  index: number;
  layout?: SourceChartLayout;
  order: number;
  pointOverrides?: SourceChartPointOverride[];
  sourceOrdinal: number;
}
export interface SourceChartChannel {
  cache?: SourceChartCache | null;
  formula?: string | null;
  literalText?: string | null;
  /**
   * Native container name; distinguishes reference, literal and title text.
   */
  nativeKind: string;
  role: ChartChannelRole;
  sourceOrdinal: number;
}
export interface SourceChartCache {
  declaredPointCount?: number | null;
  formatCode?: string | null;
  kind: ChartCacheKind;
  /**
   * Source level order; no expansion of sparse point indices or missing values.
   */
  levels: SourceChartPoint[][];
  sourceOrdinal: number;
}
export interface SourceChartPoint {
  formatCode?: string | null;
  index: number;
  sourceOrdinal: number;
  /**
   * Exact lexical value. Missing v, empty v, zero and errors remain distinct.
   */
  value?: string | null;
}
export interface SourceChartPointOverride {
  index: number;
  layout: SourceChartLayout;
  sourceOrdinal: number;
}
export interface SourceColorMapRef {
  part: string;
  sourceOrdinal: number;
}
export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}
export interface ChartPaintDeclaration {
  blackWhiteMode?: NativeBlackWhiteMode | null;
  /**
   * Every declared fill/line color is evaluated once, without quantized reuse.
   */
  colors: ChartPaintColor[];
  effectNodes: {
    [k: string]: SourceEffectNode | undefined;
  };
  effects?: SourceEffectProperties | null;
  fill?: SourceFill | null;
  line?: SourceLine | null;
  /**
   * Geometry, extensions and unknown attributes remain source-bound.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}
export interface ChartPaintColor {
  dependencies: ColorDependency[];
  notices: ColorNotice[];
  outcome: ColorSample;
  sourceOrdinal: number;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^\d+$".
 */
export interface SourceEffectNode {
  definition: SourceEffectDefinition;
  /**
   * Only this node's uninterpreted properties. Children have their own nodes.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}
export interface SourceColor {
  sourceOrdinal: number;
  /**
   * XML application order, including repeated transforms.
   */
  transforms: SourceColorTransform[];
  value: SourceColorValue;
}
export interface SourceFill {
  definition: SourceFillDefinition;
  /**
   * Physical nodes in the immutable source part. Consumers must resolve or
   * diagnose these before claiming a complete brush.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}
export interface SourceFillRect {
  bottom?: NativePercentage | null;
  left?: NativePercentage | null;
  right?: NativePercentage | null;
  sourceOrdinal: number;
  top?: NativePercentage | null;
}
export interface SourceGradientStops {
  /**
   * Native order is significant, including repeated stop positions.
   */
  entries: SourceGradientStop[];
  sourceOrdinal: number;
}
export interface SourceGradientStop {
  color: SourceColor;
  position: NativePercentage;
  sourceOrdinal: number;
}
export interface SourceFillColor {
  color: SourceColor;
  sourceOrdinal: number;
}
export interface SourceFillBlip {
  compression?: NativeBlipCompression | null;
  effectNodes?: number[];
  /**
   * Relationship IDs only: the core never opens a network URL or file path.
   */
  embed?: string | null;
  link?: string | null;
  /**
   * Uninterpreted properties only; known effects use the part catalog.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}
export interface SourceEffectProperties {
  definition: SourceEffectPropertiesDefinition;
  retainedOrdinals: number[];
  sourceOrdinal: number;
}
export interface SourceLine {
  alignment?: NativePenAlignment | null;
  cap?: NativeLineCap | null;
  compound?: NativeCompoundLine | null;
  dash?: SourceLineDash | null;
  fill?: SourceLineFill | null;
  head?: SourceLineEnd | null;
  join?: SourceLineJoin | null;
  /**
   * Unsupported attributes/extensions bind to their owning elements. They
   * cannot be discarded or counted as resolved line semantics.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
  tail?: SourceLineEnd | null;
  width?: Emu | null;
}
export interface SourceDashStop {
  dash: NativePercentage;
  sourceOrdinal: number;
  space: NativePercentage;
}
export interface SourceGradientFill {
  flip?: NativeTileFlip | null;
  rotateWithShape?: boolean | null;
  shade?: SourceGradientShade | null;
  stops?: SourceGradientStops | null;
  tileRect?: SourceFillRect | null;
}
export interface SourcePatternFill {
  background?: SourceFillColor | null;
  foreground?: SourceFillColor | null;
  preset?: NativePattern | null;
}
export interface SourceLineEnd {
  kind?: NativeLineEnd | null;
  length?: NativeLineEndSize | null;
  sourceOrdinal: number;
  width?: NativeLineEndSize | null;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface ChartThemeOverride {
  part: string;
  sha256: Digest;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
