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
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
