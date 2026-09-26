/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceLineQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Explicit interpretation of the documented multi-pass DrawingML rules.
 * This is not a certificate for a particular Office/WPS version or renderer.
 *
 * This interface was referenced by `SourceLineQuery`'s JSON-Schema
 * via the `definition` "LineProfile".
 */
export type LineProfile = "ms-oi29500-lines-2024-draft-v1";

export interface SourceLineQuery {
  expectedSourceSha256: Digest;
  objects: number[];
  profile: LineProfile;
  surface: string;
}
