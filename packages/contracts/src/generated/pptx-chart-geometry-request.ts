/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `SourceCircularRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;
/**
 * This interface was referenced by `SourceCircularRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourceCircularRequest`'s JSON-Schema
 * via the `definition` "NegativeWeights".
 */
export type NegativeWeights = "reject" | "absoluteMagnitude";
/**
 * This interface was referenced by `SourceCircularRequest`'s JSON-Schema
 * via the `definition` "SourceCircularProfile".
 */
export type SourceCircularProfile = "source-cache-declared-circular-plot-v1-draft";

export interface SourceCircularRequest {
  center: Point;
  coordinateTolerance: FixedQ32;
  expectedSourceSha256: Digest;
  negativeWeights: NegativeWeights;
  object: SourceObjectRef;
  outerRadius: FixedQ32;
  plotSourceOrdinal: number;
  profile: SourceCircularProfile;
}
/**
 * Resolved plot geometry, not the graphicFrame's outer box or a guessed
 * automatic layout. Labels/legend reserve space in their own layout stage.
 */
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * This interface was referenced by `SourceCircularRequest`'s JSON-Schema
 * via the `definition` "SourceObjectRef".
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
/**
 * This interface was referenced by `SourceCircularRequest`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}
