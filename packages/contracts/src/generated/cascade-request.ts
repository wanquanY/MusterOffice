/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "Direction".
 */
export type Direction = "leftToRight" | "rightToLeft" | "topToBottom" | "bottomToTop";

export interface CascadeRequest {
  fonts: CascadeFont[];
  items: CascadeItem[];
  text: string;
}
/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 *
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "CascadeFont".
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
/**
 * One indivisible shaping item. The caller supplies script/direction/language
 * itemization; this layer never invents script or splits a shaping dependency.
 *
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "CascadeItem".
 */
export interface CascadeItem {
  beginningOfText: boolean;
  candidates: FontCandidate[];
  direction: Direction;
  end: number;
  endOfText: boolean;
  features: ShapeFeature[];
  language: string;
  maxGlyphs: number;
  script: string;
  start: number;
  suppressDottedCircle: boolean;
}
/**
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "FontCandidate".
 */
export interface FontCandidate {
  font: number;
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
/**
 * This interface was referenced by `CascadeRequest`'s JSON-Schema
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
