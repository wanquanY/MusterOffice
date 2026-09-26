/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type BidiAnalysisResponse =
  | {
      characters: BidiCharacterProperties[];
      paragraphs: BidiParagraphResult[];
      status: "analyzed";
      unicodeVersion: string;
    }
  | {
      code: BidiFailureCode;
      message: string;
      status: "error";
    };
export type BidiClass =
  | "L"
  | "R"
  | "AL"
  | "EN"
  | "ES"
  | "ET"
  | "AN"
  | "CS"
  | "NSM"
  | "BN"
  | "B"
  | "S"
  | "WS"
  | "ON"
  | "LRE"
  | "LRO"
  | "RLE"
  | "RLO"
  | "PDF"
  | "LRI"
  | "RLI"
  | "FSI"
  | "PDI";
export type BidiFailureCode = "INPUT_INVALID" | "LIMIT_EXCEEDED" | "CANCELLED";

export interface BidiCharacterProperties {
  codepoint: number;
  properties: BidiProperties;
}
export interface BidiProperties {
  bracket?: BidiBracket | null;
  class: BidiClass;
  mirrored: boolean;
  /**
   * Informative mapping, not a direction-independent character replacement.
   */
  mirroringGlyph?: number | null;
}
export interface BidiBracket {
  isOpen: boolean;
  normalizedOpening: number;
  paired: number;
}
export interface BidiParagraphResult {
  lines: BidiLine[];
  paragraphLevel: number;
  profile: string;
  /**
   * I1/I2 result, before line-specific L1 resets. None means removed by X9.
   */
  resolvedLevels: (number | null)[];
}
export interface BidiLine {
  end: TextBoundary;
  /**
   * L1-adjusted levels, one per scalar in this line. X9 entries remain None.
   */
  levels: (number | null)[];
  start: TextBoundary;
  /**
   * L2 visual-to-logical scalar indices relative to the full paragraph.
   * X9 entries are omitted. This is not a glyph/cluster painting order (L3/L4).
   */
  visualOrder: number[];
}
export interface TextBoundary {
  scalarOffset: number;
  utf16Offset: number;
  utf8Offset: number;
}
