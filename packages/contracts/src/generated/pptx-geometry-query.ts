/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceGeometryQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourceGeometryQuery`'s JSON-Schema
 * via the `definition` "GeometryProfile".
 */
export type GeometryProfile = "ecma376-2016-ms-presets-draft-v2";

export interface SourceGeometryQuery {
  expectedSourceSha256: Digest;
  objects: number[];
  profile: GeometryProfile;
  surface: string;
}
