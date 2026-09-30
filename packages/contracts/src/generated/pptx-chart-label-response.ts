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
export interface PptxChartLabelFailure {
  code: PptxFailureCode;
  message: string;
  pointIndex?: number | null;
  seriesIndex?: number | null;
  sourceOrdinal?: number | null;
}
