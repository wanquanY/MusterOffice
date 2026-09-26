/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ShapeResponse =
  | {
      status: "shaped";
      text: ShapedText;
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

export interface ShapedText {
  faceIndex: number;
  fontSha256: Digest;
  positionUnitsPerEm: number;
  profile: string;
  runs: ShapedRun[];
  unitsPerEm: number;
}
export interface ShapedRun {
  direction: Direction;
  effectiveVariations: EffectiveVariation[];
  end: number;
  glyphs: ShapedGlyph[];
  /**
   * A computational success may still contain missing glyphs; never hide it.
   */
  missingGlyphClusters: number[];
  start: number;
}
export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
}
export interface ShapedGlyph {
  cluster: number;
  glyphId: number;
  safeToInsertTatweel: boolean;
  unsafeToBreak: boolean;
  unsafeToConcat: boolean;
  xAdvance: number;
  xOffset: number;
  yAdvance: number;
  yOffset: number;
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
