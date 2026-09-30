/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceChartLabelRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourceChartLabelRequest`'s JSON-Schema
 * via the `definition` "NegativeWeights".
 */
export type NegativeWeights = "reject" | "absoluteMagnitude";
/**
 * This interface was referenced by `SourceChartLabelRequest`'s JSON-Schema
 * via the `definition` "ChartLabelProfile".
 */
export type ChartLabelProfile = "source-chart-label-bindings-v1-draft";

export interface SourceChartLabelRequest {
  expectedSourceSha256: Digest;
  negativeWeights: NegativeWeights;
  object: SourceObjectRef;
  plotSourceOrdinal: number;
  profile: ChartLabelProfile;
  /**
   * Explicit stable identities; no allocation of a dense array from sparse idx.
   */
  targets: ChartLabelTarget[];
}
/**
 * This interface was referenced by `SourceChartLabelRequest`'s JSON-Schema
 * via the `definition` "SourceObjectRef".
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
/**
 * This interface was referenced by `SourceChartLabelRequest`'s JSON-Schema
 * via the `definition` "ChartLabelTarget".
 */
export interface ChartLabelTarget {
  pointIndex: number;
  seriesIndex: number;
}
