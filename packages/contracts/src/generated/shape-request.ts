/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "ClusterLevel".
 */
export type ClusterLevel = "monotoneGraphemes" | "monotoneCharacters" | "characters" | "graphemes";
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "Direction".
 */
export type Direction = "leftToRight" | "rightToLeft" | "topToBottom" | "bottomToTop";
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "Ignorables".
 */
export type Ignorables = "default" | "preserve" | "remove";

export interface ShapeRequest {
  expectedSha256: Digest;
  faceIndex: number;
  runs: ShapeRun[];
  /**
   * Logical text including context, with no implicit normalization.
   */
  text: string;
}
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "ShapeRun".
 */
export interface ShapeRun {
  clusterLevel: ClusterLevel;
  direction: Direction;
  end: number;
  features: ShapeFeature[];
  flags: ShapeFlags;
  language: string;
  maxGlyphs: number;
  script: string;
  /**
   * Half-open Unicode scalar range in the request text (not UTF-16).
   */
  start: number;
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "ShapeFeature".
 */
export interface ShapeFeature {
  /**
   * None means through the end of the full logical text.
   */
  end?: number | null;
  start: number;
  tag: string;
  value: number;
}
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "ShapeFlags".
 */
export interface ShapeFlags {
  beginningOfText: boolean;
  endOfText: boolean;
  ignorables: Ignorables;
  safeToInsertTatweel: boolean;
  suppressDottedCircle: boolean;
  unsafeToConcat: boolean;
}
/**
 * This interface was referenced by `ShapeRequest`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
