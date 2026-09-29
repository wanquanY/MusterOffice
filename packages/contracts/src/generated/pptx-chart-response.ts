/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxChartsResponse =
  | {
      charts: SourceCharts;
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
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceCharts {
  bindings: SourceChartBinding[];
  /**
   * Shared native chart parts are inspected once, indexed by bindings.
   */
  charts: SourceChartPart[];
  sourceSha256: Digest;
  surface: string;
}
export interface SourceChartBinding {
  chart: number;
  object: SourceObjectRef;
  relationshipId: string;
  sourceOrdinal: number;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface SourceChartPart {
  byteLength: ByteLength;
  compatibility: SourceCompatibility;
  /**
   * Cache declarations are snapshots, not recalculated workbook values.
   */
  dataAuthority: "sourceCacheSnapshot";
  externalData?: SourceChartExternalData | null;
  part: string;
  plots: SourceChartPlot[];
  sha256: Digest;
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
  order: number;
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
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
