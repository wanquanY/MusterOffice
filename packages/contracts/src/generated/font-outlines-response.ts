/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type FontOutlinesResponse =
  | {
      outlines: FontOutlinesResult;
      status: "outlined";
    }
  | {
      error: ShapeFailure;
      status: "error";
    };
export type Digest = string;
export type OutlineCommand =
  | {
      kind: "move";
      to: OutlinePoint;
    }
  | {
      kind: "line";
      to: OutlinePoint;
    }
  | {
      control: OutlinePoint;
      kind: "quadratic";
      to: OutlinePoint;
    }
  | {
      control1: OutlinePoint;
      control2: OutlinePoint;
      kind: "cubic";
      to: OutlinePoint;
    }
  | {
      kind: "close";
    };
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

export interface FontOutlinesResult {
  /**
   * Presence only: does not imply that a particular glyph uses these tables.
   */
  colorTables: string[];
  faceIndex: number;
  fontSha256: Digest;
  instances: OutlinedInstance[];
  positionUnitsPerEm: number;
  profile: string;
  unitsPerEm: number;
}
export interface OutlinedInstance {
  effectiveVariations: EffectiveVariation[];
  glyphs: GlyphOutline[];
}
export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
}
export interface GlyphOutline {
  glyphId: number;
  /**
   * None is unavailable/failed, Some([]) is a valid empty outline (e.g. space).
   */
  path?: OutlineCommand[] | null;
}
export interface OutlinePoint {
  x: number;
  y: number;
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
