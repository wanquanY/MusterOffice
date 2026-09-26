/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * An explicit interpretation, not a certificate for any Office/WPS version.
 *
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "FillProfile".
 */
export type FillProfile = "ms-oi29500-fills-2024-draft-v1";
/**
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "FillTarget".
 */
export type FillTarget =
  | {
      kind: "object";
      nativeId: number;
    }
  | {
      kind: "line";
      nativeId: number;
    }
  | {
      kind: "picture";
      nativeId: number;
    }
  | {
      kind: "rootGroup";
    }
  | {
      kind: "background";
    };
/**
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "RadialLayoutProfile".
 */
export type RadialLayoutProfile =
  "drawingml-circle-path-bounds-q96-v1-draft" | "drawingml-circle-anchor-focus-q96-v2-draft";
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;

export interface PptxRadialLayoutRequest {
  fills: SourceFillQuery;
  options: RadialLayoutOptions;
  profile: RadialLayoutProfile;
}
/**
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "SourceFillQuery".
 */
export interface SourceFillQuery {
  expectedSourceSha256: Digest;
  profile: FillProfile;
  surface: string;
  targets: FillTarget[];
}
/**
 * This interface was referenced by `PptxRadialLayoutRequest`'s JSON-Schema
 * via the `definition` "RadialLayoutOptions".
 */
export interface RadialLayoutOptions {
  /**
   * Per-axis Q32 local EMU path/bounds tolerance, relative to evaluated guides.
   * Derived focus errors may be larger. Excludes page placement and application differences.
   */
  coordinateTolerance: string;
}
