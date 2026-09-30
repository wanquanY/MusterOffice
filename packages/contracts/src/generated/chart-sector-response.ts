/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ChartSectorsResponse =
  | {
      layout: SectorLayout;
      status: "computed";
    }
  | {
      error: ChartSectorFailure;
      status: "error";
    };
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type ChartSectorFailureCode = "inputInvalid" | "limitExceeded" | "cancelled";

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
export interface ChartSectorFailure {
  code: ChartSectorFailureCode;
  message: string;
  pointIndex?: number | null;
}
