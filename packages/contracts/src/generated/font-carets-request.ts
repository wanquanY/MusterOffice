/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `FontCaretsRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `FontCaretsRequest`'s JSON-Schema
 * via the `definition` "Direction".
 */
export type Direction = "leftToRight" | "rightToLeft" | "topToBottom" | "bottomToTop";

export interface FontCaretsRequest {
  expectedSha256: Digest;
  faceIndex: number;
  instances: FontCaretsInstance[];
}
/**
 * This interface was referenced by `FontCaretsRequest`'s JSON-Schema
 * via the `definition` "FontCaretsInstance".
 */
export interface FontCaretsInstance {
  direction: Direction;
  glyphIds: number[];
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `FontCaretsRequest`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
