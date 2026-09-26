/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `BidiAnalysisRequest`'s JSON-Schema
 * via the `definition` "ParagraphDirection".
 */
export type ParagraphDirection = "autoLeftToRight" | "leftToRight" | "rightToLeft";

export interface BidiAnalysisRequest {
  characters: number[];
  paragraphs: BidiParagraphRequest[];
}
/**
 * This interface was referenced by `BidiAnalysisRequest`'s JSON-Schema
 * via the `definition` "BidiParagraphRequest".
 */
export interface BidiParagraphRequest {
  direction: ParagraphDirection;
  /**
   * Unicode scalar line ends supplied by layout, increasing through text end.
   * Empty means one complete line, not automatic width-based wrapping.
   */
  lineEnds: number[];
  text: string;
}
