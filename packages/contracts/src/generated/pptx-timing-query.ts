/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceTimingQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface SourceTimingQuery {
  expectedSourceSha256: Digest;
  slide: string;
}
