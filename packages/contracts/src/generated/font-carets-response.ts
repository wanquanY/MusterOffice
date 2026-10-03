/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type FontCaretsResponse =
  | {
      carets: FontCaretsResult;
      status: "queried";
    }
  | {
      error: ShapeFailure;
      status: "error";
    };
export type Digest = string;
export type Direction = "leftToRight" | "rightToLeft" | "topToBottom" | "bottomToTop";
export type ShapeFailureCode =
  | "INPUT_INVALID"
  | "FONT_INVALID"
  | "UNSUPPORTED"
  | "RESOURCE_CONFLICT"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";
export type FontStyle = "regular" | "bold" | "italic" | "boldItalic";
export type FontSelectionReason = "unmappedTypeface" | "missingStyle";

export interface FontCaretsResult {
  faceIndex: number;
  fontSha256: Digest;
  instances: MeasuredCarets[];
  positionUnitsPerEm: number;
  profile: string;
  unitsPerEm: number;
}
export interface MeasuredCarets {
  direction: Direction;
  effectiveVariations: EffectiveVariation[];
  glyphs: GlyphCarets[];
}
export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
}
export interface GlyphCarets {
  glyphId: number;
  /**
   * GDEF positions in the font's order, before shaping placement/kerning.
   * Empty means the font supplies no carets; zero is a real position.
   */
  positions: number[];
}
export interface ShapeFailure {
  code: ShapeFailureCode;
  fontSelection?: FontSelectionFailure | null;
  message: string;
}
export interface FontSelectionFailure {
  fontStyle: FontStyle;
  reason: FontSelectionReason;
  /**
   * Index in the actual ManifestParagraphInput.styles, not a native run id.
   */
  style: number;
  typeface: string;
}
