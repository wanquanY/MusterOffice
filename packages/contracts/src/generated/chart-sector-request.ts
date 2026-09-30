/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SectorRequest`'s JSON-Schema
 * via the `definition` "SectorDirection".
 */
export type SectorDirection = "clockwise" | "counterclockwise";
/**
 * This interface was referenced by `SectorRequest`'s JSON-Schema
 * via the `definition` "NegativeWeights".
 */
export type NegativeWeights = "reject" | "absoluteMagnitude";
/**
 * Exact finite decimal spelling, never a JSON floating point number. Semantic validation limits normalized decimal exponent to -4096..4096 and lexical UTF-8 bytes to 1024. Null, blanks, error tokens and infinities are not numeric weights.
 *
 * This interface was referenced by `SectorRequest`'s JSON-Schema
 * via the `definition` "ChartDecimalNumber".
 */
export type ChartDecimalNumber = string;
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `SectorRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;

export interface SectorRequest {
  direction: SectorDirection;
  negativeWeights: NegativeWeights;
  /**
   * Q32 turns from twelve o'clock, in [0, 1). No degree/EMU conversion here.
   */
  startTurn: string;
  /**
   * Explicit drawing order. Stable point indices need not be contiguous.
   */
  weights: SectorWeight[];
}
/**
 * This interface was referenced by `SectorRequest`'s JSON-Schema
 * via the `definition` "SectorWeight".
 */
export interface SectorWeight {
  pointIndex: number;
  value: ChartDecimalNumber;
}
