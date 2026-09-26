/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "ParagraphDirection".
 */
export type ParagraphDirection = "autoLeftToRight" | "leftToRight" | "rightToLeft";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface ParagraphShapeRequest {
  direction: ParagraphDirection;
  fonts: CascadeFont[];
  spans: StyleSpan[];
  styles: ParagraphTextStyle[];
  text: string;
}
/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 *
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "CascadeFont".
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
/**
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "StyleSpan".
 */
export interface StyleSpan {
  end: number;
  style: number;
}
/**
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "ParagraphTextStyle".
 */
export interface ParagraphTextStyle {
  candidates: FontCandidate[];
  features: ShapeFeature[];
  language: string;
  maxGlyphs: number;
  suppressDottedCircle: boolean;
}
/**
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
 * via the `definition` "FontCandidate".
 */
export interface FontCandidate {
  font: number;
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
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
 * This interface was referenced by `ParagraphShapeRequest`'s JSON-Schema
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
