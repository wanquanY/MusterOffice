/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceNativePathsQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourceNativePathsQuery`'s JSON-Schema
 * via the `definition` "GeometryProfile".
 */
export type GeometryProfile = "ecma376-2016-ms-presets-draft-v2";
/**
 * This interface was referenced by `SourceNativePathsQuery`'s JSON-Schema
 * via the `definition` "NativePathProfile".
 */
export type NativePathProfile = "drawingml-polar-arcs-q96-hermite-v1-draft";
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `SourceNativePathsQuery`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;

export interface SourceNativePathsQuery {
  geometry: SourceGeometryQuery;
  options: NativePathOptions;
}
/**
 * This interface was referenced by `SourceNativePathsQuery`'s JSON-Schema
 * via the `definition` "SourceGeometryQuery".
 */
export interface SourceGeometryQuery {
  expectedSourceSha256: Digest;
  objects: number[];
  profile: GeometryProfile;
  surface: string;
}
/**
 * This interface was referenced by `SourceNativePathsQuery`'s JSON-Schema
 * via the `definition` "NativePathOptions".
 */
export interface NativePathOptions {
  /**
   * Per-axis local shape EMU, Q32. Does not include upstream guide error,
   * page transforms, raster conversion, coverage or application differences.
   */
  coordinateTolerance: string;
  profile: NativePathProfile;
}
