/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ChartGeometryResponse =
  | {
      geometry: ChartGeometry;
      status: "computed";
    }
  | {
      error: ChartGeometryFailure;
      status: "error";
    };
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
export type ChartGeometryFailureCode =
  "inputInvalid" | "limitExceeded" | "precisionExceeded" | "numericRange" | "cancelled";

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
export interface ChartGeometryFailure {
  code: ChartGeometryFailureCode;
  message: string;
  pointIndex?: number | null;
}
