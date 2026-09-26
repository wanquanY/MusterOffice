/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "OverflowPolicy".
 */
export type OverflowPolicy = "keepUnbreakable" | "emergencyGrapheme";
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "ParagraphDirection".
 */
export type ParagraphDirection = "autoLeftToRight" | "leftToRight" | "rightToLeft";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "LineSpacing".
 */
export type LineSpacing =
  | {
      kind: "natural";
    }
  | {
      height: Emu;
      kind: "exact";
    }
  | {
      height: Emu;
      kind: "atLeast";
    }
  | {
      heights: FixedQ32[];
      kind: "styleMaximum";
    };
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 *
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "Emu".
 */
export type Emu = string;
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;
/**
 * Keep legacy integer-EMU inputs while allowing native percentages to reach
 * layout without an intermediate integer-EMU rounding. Equality is numeric.
 *
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "BaselineShift".
 */
export type BaselineShift =
  | Emu
  | {
      q32: FixedQ32;
    };

export interface ParagraphLayoutRequest {
  overflow: OverflowPolicy;
  paragraph: ParagraphShapeRequest;
  spacing: LineSpacing;
  strutStyle: number;
  styles: GeometryStyle[];
  width: Emu;
}
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "ParagraphShapeRequest".
 */
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
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "CascadeFont".
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "StyleSpan".
 */
export interface StyleSpan {
  end: number;
  style: number;
}
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
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
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "FontCandidate".
 */
export interface FontCandidate {
  font: number;
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
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
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
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
 * This interface was referenced by `ParagraphLayoutRequest`'s JSON-Schema
 * via the `definition` "GeometryStyle".
 */
export interface GeometryStyle {
  /**
   * Positive values raise the baseline in the y-down output coordinate system.
   */
  baselineShift:
    | Emu
    | {
        q32: FixedQ32;
      };
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  clusterSpacing?: string;
  fontSize: Emu;
}
