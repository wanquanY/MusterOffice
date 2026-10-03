/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxChartGeometryResponse =
  | {
      geometry: SourceCircularGeometry;
      status: "compiled";
    }
  | {
      error: PptxChartGeometryFailure;
      status: "error";
    };
export type Digest = string;
export type ChartDataAuthority = "sourceCacheSnapshot";
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
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type SourceCircularProfile = "source-cache-declared-circular-plot-v1-draft";
export type FillRule = "nonzero" | "evenodd";
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type PathCommand =
  | {
      kind: "move";
      to: Point;
    }
  | {
      kind: "line";
      to: Point;
    }
  | {
      control: Point;
      kind: "quadratic";
      to: Point;
    }
  | {
      control1: Point;
      control2: Point;
      kind: "cubic";
      to: Point;
    }
  | {
      kind: "close";
    };
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
  | "extensions"
  | "dataLabel"
  | "legendEntry"
  | "layout"
  | "manualLayout"
  | "textSource"
  | "leaderLines";
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
  | "smooth"
  | "labelPosition"
  | "showLegendKey"
  | "showValue"
  | "showCategoryName"
  | "showSeriesName"
  | "showPercent"
  | "showBubbleSize"
  | "separator"
  | "showLeaderLines"
  | "legendPosition"
  | "overlay"
  | "layoutTarget"
  | "xMode"
  | "yMode"
  | "widthMode"
  | "heightMode"
  | "x"
  | "y"
  | "width"
  | "height";
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceCircularGeometry {
  chartPart: string;
  chartSha256: Digest;
  dataAuthority: ChartDataAuthority;
  externalData?: SourceChartExternalData | null;
  firstSliceDegrees: number;
  holePercent: number;
  nativeKind: string;
  object: SourceObjectRef;
  plotSourceOrdinal: number;
  profile: SourceCircularProfile;
  /**
   * Source series order, with the innermost doughnut ring first. Stable series
   * indices are retained independently of physical XML or drawing order.
   */
  series: SourceCircularSeries[];
  sourceSha256: Digest;
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
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface SourceCircularSeries {
  /**
   * Maximum complete per-axis bound across all paths, including source error.
   */
  coordinateErrorBound: string;
  formula?: string | null;
  geometry: ChartGeometry;
  index: number;
  innerRadius: FixedQ32;
  layout: SourceChartLayout;
  order: number;
  outerRadius: FixedQ32;
  pointOverrides: SourceChartPointOverride[];
  points: SourceCircularPoint[];
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  sourceGeometryErrorBound: string;
  sourceOrdinal: number;
  valuesSourceOrdinal: number;
}
export interface ChartGeometry {
  fillRule: FillRule;
  layout: SectorLayout;
  paths: ChartSectorPath[];
  profile: string;
  work: ChartGeometryWork;
}
export interface SectorLayout {
  /**
   * Conservative endpoint error in raw Q32 turn units, relative to start_turn.
   * For a sweep, combine the two endpoint errors. No geometric error claimed.
   */
  endpointErrorBound: string;
  profile: string;
  sectors: Sector[];
  work: SectorWork;
  zeroTotal: boolean;
}
export interface Sector {
  endTurn: FixedQ32;
  pointIndex: number;
  /**
   * A positive weight survives in the plan even if its Q32 endpoints coincide.
   */
  positiveBelowResolution: boolean;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  startTurn: string;
  zeroWeight: boolean;
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
export interface ChartSectorPath {
  angularErrorBound: FixedQ32;
  arcSegments: number;
  /**
   * Empty for zero-weight sectors. Positive sub-resolution sectors fail with
   * Precision rather than disappearing from a successful geometry result.
   * Full rings use opposite winding subpaths, without a radial stroke seam.
   */
  commands: PathCommand[];
  coordinateErrorBound: FixedQ32;
  curveErrorBound: FixedQ32;
  numericErrorBound: FixedQ32;
  pointIndex: number;
}
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
export interface ChartGeometryWork {
  arcSegments: number;
  commands: number;
  paths: number;
  /**
   * Bounded arithmetic stages; each trigonometric evaluation charges 24.
   */
  steps: number;
}
export interface SourceChartLayout {
  marker?: SourceChartMarker | null;
  /**
   * Complex markup remains bound to its original part and physical ordinal.
   */
  markup: SourceChartMarkup[];
  /**
   * Source order; missing property, missing val and explicit lexical val differ.
   */
  properties: SourceChartProperty[];
  /**
   * Known elements carrying attributes that this declaration model does not interpret.
   */
  retainedAttributeOrdinals?: number[];
  unrecognizedChildren?: SourceChartUnknown[];
}
export interface SourceChartMarker {
  retainedOrdinals: number[];
  size?: number | null;
  sourceOrdinal: number;
  symbol?: string | null;
}
export interface SourceChartMarkup {
  kind: ChartMarkupKind;
  sourceOrdinal: number;
}
export interface SourceChartProperty {
  kind: ChartPropertyKind;
  sourceOrdinal: number;
  /**
   * XML val attribute (or separator character data), without schema defaulting.
   */
  value?: string | null;
}
export interface SourceChartUnknown {
  localName: string;
  namespace: string;
  sourceOrdinal: number;
}
export interface SourceChartPointOverride {
  index: number;
  layout: SourceChartLayout;
  sourceOrdinal: number;
}
export interface SourceCircularPoint {
  formatCode?: string | null;
  index: number;
  sourceOrdinal: number;
}
export interface PptxChartGeometryFailure {
  code: PptxFailureCode;
  message: string;
  pointIndex?: number | null;
  seriesIndex?: number | null;
  sourceOrdinal?: number | null;
}
