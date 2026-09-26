/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourcePlacementQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourcePlacementQuery`'s JSON-Schema
 * via the `definition` "SourcePlacementProfile".
 */
export type SourcePlacementProfile = "drawingml-source-sector-scale-q96-v1-draft";

export interface SourcePlacementQuery {
  expectedSourceSha256: Digest;
  objects: number[];
  profile: SourcePlacementProfile;
  surface: string;
}
