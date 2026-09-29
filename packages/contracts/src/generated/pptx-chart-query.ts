/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceChartQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface SourceChartQuery {
  expectedSourceSha256: Digest;
  /**
   * Exact physical surface; inherited chart placement is a separate calculation.
   */
  surface: string;
}
