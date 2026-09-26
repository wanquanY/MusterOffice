/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `FontOutlinesRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface FontOutlinesRequest {
  expectedSha256: Digest;
  faceIndex: number;
  instances: OutlineInstance[];
}
/**
 * This interface was referenced by `FontOutlinesRequest`'s JSON-Schema
 * via the `definition` "OutlineInstance".
 */
export interface OutlineInstance {
  glyphIds: number[];
  /**
   * Aggregate across all glyphs of this instance, including discarded prefixes.
   */
  maxCommands: number;
  /**
   * HarfBuzz's weighted interpreter budget, not CPU instructions or time.
   */
  maxOperations: number;
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `FontOutlinesRequest`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
