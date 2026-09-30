/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxChartLabelsResponse =
  | {
      labels: SourceChartLabels;
      status: "planned";
    }
  | {
      error: PptxChartLabelFailure;
      status: "error";
    };
export type Digest = string;
export type ChartDataAuthority = "sourceCacheSnapshot";
export type ChartLabelComponent =
  | {
      channelSourceOrdinal: number;
      kind: "text";
      role: ChartLabelFlag;
      sourceOrdinal: number;
      value: string;
    }
  | {
      channelSourceOrdinal: number;
      format?: ChartLabelDataFormat | null;
      kind: "number";
      role: ChartLabelFlag;
      sourceOrdinal: number;
      value: ChartDecimalNumber;
    }
  | {
      format?: ChartLabelDataFormat | null;
      kind: "percent";
      normalization: number;
      pointIndex: number;
    };
export type ChartLabelFlag = "legendKey" | "value" | "categoryName" | "seriesName" | "percent" | "bubbleSize";
/**
 * Exact finite decimal spelling, never a JSON floating point number. Semantic validation limits normalized decimal exponent to -4096..4096 and lexical UTF-8 bytes to 1024. Null, blanks, error tokens and infinities are not numeric weights.
 */
export type ChartDecimalNumber = string;
export type ChartLabelPosition = "bestFit" | "b" | "ctr" | "inBase" | "inEnd" | "l" | "outEnd" | "r" | "t";
export type ChartLabelSeparator =
  | {
      kind: "declared";
      sourceOrdinal: number;
      value: string;
    }
  | {
      kind: "commaDefault";
    }
  | {
      kind: "pieCategoryPercentLineBreak";
    };
export type NegativeWeights = "reject" | "absoluteMagnitude";
export type ChartLabelProfile = "source-chart-label-bindings-v1-draft";
export type ChartTextOutcome =
  | {
      status: "cascaded";
      text: ChartTextCascade;
    }
  | {
      reason: TextCascadeUnresolved;
      status: "unresolved";
    };
export type NativeTextAlign = "l" | "ctr" | "r" | "just" | "justLow" | "dist" | "thaiDist";
/**
 * Native coordinate: bounded integer EMU or exact decimal universal measure.
 */
export type NativeCoordinate = string;
export type NativeTextFontAlign = "auto" | "t" | "ctr" | "base" | "b";
export type NativeTextElement =
  | (
      | "txBody"
      | "txStyles"
      | "defaultTextStyle"
      | "titleStyle"
      | "bodyStyle"
      | "otherStyle"
      | "lstStyle"
      | "bodyPr"
      | "p"
      | "pPr"
      | "defPPr"
      | "lvl1pPr"
      | "lvl2pPr"
      | "lvl3pPr"
      | "lvl4pPr"
      | "lvl5pPr"
      | "lvl6pPr"
      | "lvl7pPr"
      | "lvl8pPr"
      | "lvl9pPr"
      | "r"
      | "br"
      | "fld"
      | "t"
      | "rPr"
      | "defRPr"
      | "endParaRPr"
      | "noAutofit"
      | "normAutofit"
      | "spAutoFit"
      | "lnSpc"
      | "spcBef"
      | "spcAft"
      | "spcPct"
      | "spcPts"
      | "buClrTx"
      | "buClr"
      | "buSzTx"
      | "buSzPct"
      | "buSzPts"
      | "buFontTx"
      | "buFont"
      | "buNone"
      | "buAutoNum"
      | "buChar"
      | "tabLst"
      | "tab"
      | "latin"
      | "ea"
      | "cs"
      | "sym"
      | "fontRef"
      | "highlight"
      | "uLnTx"
      | "uLn"
      | "uFillTx"
      | "uFill"
      | "hlinkClick"
      | "hlinkMouseOver"
      | "rtl"
      | "noFill"
      | "solidFill"
      | "gradFill"
      | "blipFill"
      | "pattFill"
      | "grpFill"
      | "ln"
      | "effectLst"
      | "effectDag"
      | "srgbClr"
      | "scrgbClr"
      | "hslClr"
      | "sysClr"
      | "schemeClr"
      | "prstClr"
    )
  | "txPr"
  | "rich"
  | "font";
export type TextStyleOrigin =
  | {
      bodySourceOrdinal: number;
      kind: "chart";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "tableStyle";
      region: TableStyleRegion;
      source: TableTextStyleSource;
      sourceOrdinal: number;
    }
  | {
      kind: "object";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      kind: "master";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "presentation";
      part: string;
      sourceOrdinal: number;
    }
  | {
      defaultKind: SourceThemeDefaultKind;
      kind: "theme";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "profileDefault";
    };
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
export type TableTextStyleSource =
  | {
      kind: "inline";
      object: SourceObjectRef;
    }
  | {
      kind: "catalog";
      part: string;
      styleId: string;
    };
export type SourceThemeDefaultKind = "txDef" | "lnDef" | "spDef";
/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;
export type NativeTextCaps = "none" | "small" | "all";
export type NativeTextPoint =
  | {
      kind: "hundredthPoints";
      value: number;
    }
  | {
      kind: "universalMeasure";
      value: NativeCoordinate;
    };
export type NativeTextStrike = "noStrike" | "sngStrike" | "dblStrike";
export type NativeTextUnderline =
  | "none"
  | "words"
  | "sng"
  | "dbl"
  | "heavy"
  | "dotted"
  | "dottedHeavy"
  | "dash"
  | "dashHeavy"
  | "dashLong"
  | "dashLongHeavy"
  | "dotDash"
  | "dotDashHeavy"
  | "dotDotDash"
  | "dotDotDashHeavy"
  | "wavy"
  | "wavyHeavy"
  | "wavyDbl";
export type SourceRunKind = "text" | "break" | "field";
export type TextCascadeUnresolved =
  | {
      kind: "chartText";
      origin: TextStyleOrigin;
      reason: ChartTextUnresolved;
    }
  | {
      kind: "tableGrid";
      reason: NativeTableGridIssue;
    }
  | {
      kind: "tableStyle";
      reason: TableStyleSelectionError;
    }
  | {
      kind: "noTextBody";
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
      object: SourceObjectRef;
    }
  | {
      kind: "retainedContent";
      origin: TextStyleOrigin;
    }
  | {
      kind: "ambiguousTemplateParagraph";
      level: number;
      object: SourceObjectRef;
    }
  | {
      kind: "fieldParagraph";
      origin: TextStyleOrigin;
    };
export type ChartTextUnresolved = "multiplePropertyParagraphs" | "propertyTextRuns" | "listStyle";
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

export interface SourceChartLabels {
  chartPart: string;
  chartSha256: Digest;
  dataAuthority: ChartDataAuthority;
  labels: ChartLabelPlan[];
  negativeWeights: NegativeWeights;
  /**
   * Each required series is normalized once. Fractions are not sector angles.
   */
  normalizations: ChartLabelNormalization[];
  object: SourceObjectRef;
  plotSourceOrdinal: number;
  profile: ChartLabelProfile;
  sourceSha256: Digest;
  /**
   * Unique native text cascades, shared by labels with the same declaration chain.
   */
  textCascades: ChartTextOutcome[];
}
export interface ChartLabelPlan {
  /**
   * Most specific first. Missing declarations do not clear inherited fields.
   */
  annotationChain: number[];
  /**
   * Named data bindings, not final display order or formatted display text.
   */
  components: ChartLabelComponent[];
  customTextSource?: number | null;
  /**
   * Office displays a legend key only alongside a selected text component or tx.
   */
  legendKeyVisible?: boolean | null;
  settings: ChartLabelSettings;
  target: ChartLabelTarget;
  /**
   * Index into text_cascades. None requires chart-style/default text resolution.
   */
  textCascade?: number | null;
}
export interface ChartLabelDataFormat {
  code: string;
  sourceOrdinal: number;
}
export interface ChartLabelSettings {
  deleted?: ChartLabelValue | null;
  flags: {
    bubbleSize?: ChartLabelValue;
    categoryName?: ChartLabelValue;
    legendKey?: ChartLabelValue;
    percent?: ChartLabelValue;
    seriesName?: ChartLabelValue;
    value?: ChartLabelValue;
  };
  layoutSourceOrdinal?: number | null;
  numberFormat?: ChartLabelNumberFormat | null;
  position?: ChartLabelValue2 | null;
  separator?: ChartLabelSeparator | null;
  shapePropertyRoots: number[];
  showLeaderLines?: ChartLabelValue | null;
  textPropertyRoots: number[];
  /**
   * Missing throughout the chain: application/chart-style defaults still needed.
   */
  unresolvedFlags: ChartLabelFlag[];
}
export interface ChartLabelValue {
  /**
   * Present CT_Boolean/numFmt with omitted val/sourceLinked uses schema true.
   */
  schemaDefaulted: boolean;
  sourceOrdinal: number;
  value: boolean;
}
export interface ChartLabelNumberFormat {
  code: string;
  sourceLinked: ChartLabelValue;
  sourceOrdinal: number;
}
export interface ChartLabelValue2 {
  /**
   * Present CT_Boolean/numFmt with omitted val/sourceLinked uses schema true.
   */
  schemaDefaulted: boolean;
  sourceOrdinal: number;
  value: ChartLabelPosition;
}
export interface ChartLabelTarget {
  pointIndex: number;
  seriesIndex: number;
}
export interface ChartLabelNormalization {
  channelSourceOrdinal: number;
  ratios: ExactWeightRatios;
  seriesIndex: number;
}
/**
 * Exact normalized magnitudes for percent labels. Denominator is stored once;
 * consumers must handle zero_total, not silently display a zero percentage.
 */
export interface ExactWeightRatios {
  denominator: string;
  numerators: ExactWeightNumerator[];
  work: SectorWork;
  zeroTotal: boolean;
}
export interface ExactWeightNumerator {
  /**
   * Unsigned decimal integer, before percentage formatting or rounding.
   */
  numerator: string;
  pointIndex: number;
}
export interface SectorWork {
  /**
   * Bounds repeated prefix/total arithmetic even if only one weight is wide.
   */
  boundaryDecimalDigits: number;
  numberBytes: number;
  points: number;
  /**
   * Sum of exact integer widths after common decimal scaling, including zeros.
   */
  scaledDecimalDigits: number;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface ChartTextCascade {
  bodySourceOrdinal: number;
  chartPart: string;
  chartSha256: Digest;
  /**
   * Declared cascade only. Chart style/default, paint, fonts and layout remain
   * separate computations; None does not mean a shape default was selected.
   */
  paragraphs: CascadedParagraph[];
  /**
   * High to low priority, excluding the body itself. These are native txPr roots.
   */
  propertyRoots: number[];
}
export interface CascadedParagraph {
  attributes: SourceTextParagraphAttributes;
  declarations: {
    bullet?: TextStyleDeclaration;
    bulletColor?: TextStyleDeclaration;
    bulletFont?: TextStyleDeclaration;
    bulletSize?: TextStyleDeclaration;
    lineSpacing?: TextStyleDeclaration;
    spaceAfter?: TextStyleDeclaration;
    spaceBefore?: TextStyleDeclaration;
    tabs?: TextStyleDeclaration;
  };
  endStyle: CascadedCharacterStyle;
  origins: {
    alignment?: TextStyleOrigin;
    defaultTabSize?: TextStyleOrigin;
    eastAsianLineBreak?: TextStyleOrigin;
    fontAlignment?: TextStyleOrigin;
    hangingPunctuation?: TextStyleOrigin;
    indent?: TextStyleOrigin;
    latinLineBreak?: TextStyleOrigin;
    leftMargin?: TextStyleOrigin;
    level?: TextStyleOrigin;
    rightMargin?: TextStyleOrigin;
    rightToLeft?: TextStyleOrigin;
  };
  runs: CascadedTextRun[];
  sourceOrdinal: number;
}
export interface SourceTextParagraphAttributes {
  alignment?: NativeTextAlign | null;
  defaultTabSize?: NativeCoordinate | null;
  eastAsianLineBreak?: boolean | null;
  fontAlignment?: NativeTextFontAlign | null;
  hangingPunctuation?: boolean | null;
  indent?: number | null;
  latinLineBreak?: boolean | null;
  leftMargin?: number | null;
  level?: number | null;
  rightMargin?: number | null;
  rightToLeft?: boolean | null;
}
/**
 * Selected whole declaration. Consumers must still resolve its native semantics,
 * retained descendants, colors, theme fonts, resources, effects and permissions.
 * The reference is meaningful only with this result's immutable SourceIndex.
 */
export interface TextStyleDeclaration {
  element: NativeTextElement;
  origin: TextStyleOrigin;
}
/**
 * Separate insertion/empty-paragraph style; never applied to existing runs.
 */
export interface CascadedCharacterStyle {
  attributes: SourceTextCharacterAttributes;
  /**
   * Empty/mutually exclusive declarations replace a slot as a whole.
   */
  declarations: {
    click?: TextStyleDeclaration;
    complexScript?: TextStyleDeclaration;
    eastAsian?: TextStyleDeclaration;
    effects?: TextStyleDeclaration;
    fill?: TextStyleDeclaration;
    highlight?: TextStyleDeclaration;
    latin?: TextStyleDeclaration;
    line?: TextStyleDeclaration;
    mouseOver?: TextStyleDeclaration;
    rightToLeft?: TextStyleDeclaration;
    symbol?: TextStyleDeclaration;
    underlineFill?: TextStyleDeclaration;
    underlineLine?: TextStyleDeclaration;
  };
  origins: {
    alternativeLanguage?: TextStyleOrigin;
    baseline?: TextStyleOrigin;
    bold?: TextStyleOrigin;
    bookmark?: TextStyleOrigin;
    caps?: TextStyleOrigin;
    dirty?: TextStyleOrigin;
    error?: TextStyleOrigin;
    italic?: TextStyleOrigin;
    kerning?: TextStyleOrigin;
    kumimoji?: TextStyleOrigin;
    language?: TextStyleOrigin;
    noProof?: TextStyleOrigin;
    normalizeHeight?: TextStyleOrigin;
    size?: TextStyleOrigin;
    smartClean?: TextStyleOrigin;
    smartId?: TextStyleOrigin;
    spacing?: TextStyleOrigin;
    strike?: TextStyleOrigin;
    underline?: TextStyleOrigin;
  };
}
/**
 * None kerning means no kerning after cascade exhaustion, not a zero threshold.
 * Language, alternative language and bookmark can also remain unspecified.
 */
export interface SourceTextCharacterAttributes {
  alternativeLanguage?: string | null;
  baseline?: NativePercentage | null;
  bold?: boolean | null;
  bookmark?: string | null;
  caps?: NativeTextCaps | null;
  dirty?: boolean | null;
  error?: boolean | null;
  italic?: boolean | null;
  kerning?: number | null;
  kumimoji?: boolean | null;
  language?: string | null;
  noProof?: boolean | null;
  normalizeHeight?: boolean | null;
  size?: number | null;
  smartClean?: boolean | null;
  smartId?: number | null;
  spacing?: NativeTextPoint | null;
  strike?: NativeTextStrike | null;
  underline?: NativeTextUnderline | null;
}
export interface CascadedTextRun {
  kind: SourceRunKind;
  /**
   * Index into SourceObject.paragraphs[paragraph]. Text is not copied here.
   */
  run: number;
  sourceOrdinal: number;
  style: CascadedCharacterStyle1;
}
export interface CascadedCharacterStyle1 {
  attributes: SourceTextCharacterAttributes;
  /**
   * Empty/mutually exclusive declarations replace a slot as a whole.
   */
  declarations: {
    click?: TextStyleDeclaration;
    complexScript?: TextStyleDeclaration;
    eastAsian?: TextStyleDeclaration;
    effects?: TextStyleDeclaration;
    fill?: TextStyleDeclaration;
    highlight?: TextStyleDeclaration;
    latin?: TextStyleDeclaration;
    line?: TextStyleDeclaration;
    mouseOver?: TextStyleDeclaration;
    rightToLeft?: TextStyleDeclaration;
    symbol?: TextStyleDeclaration;
    underlineFill?: TextStyleDeclaration;
    underlineLine?: TextStyleDeclaration;
  };
  origins: {
    alternativeLanguage?: TextStyleOrigin;
    baseline?: TextStyleOrigin;
    bold?: TextStyleOrigin;
    bookmark?: TextStyleOrigin;
    caps?: TextStyleOrigin;
    dirty?: TextStyleOrigin;
    error?: TextStyleOrigin;
    italic?: TextStyleOrigin;
    kerning?: TextStyleOrigin;
    kumimoji?: TextStyleOrigin;
    language?: TextStyleOrigin;
    noProof?: TextStyleOrigin;
    normalizeHeight?: TextStyleOrigin;
    size?: TextStyleOrigin;
    smartClean?: TextStyleOrigin;
    smartId?: TextStyleOrigin;
    spacing?: TextStyleOrigin;
    strike?: TextStyleOrigin;
    underline?: TextStyleOrigin;
  };
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
export interface PptxChartLabelFailure {
  code: PptxFailureCode;
  message: string;
  pointIndex?: number | null;
  seriesIndex?: number | null;
  sourceOrdinal?: number | null;
}
