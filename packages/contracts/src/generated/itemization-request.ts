/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ItemizationRequest`'s JSON-Schema
 * via the `definition` "ParagraphDirection".
 */
export type ParagraphDirection = "autoLeftToRight" | "leftToRight" | "rightToLeft";

export interface ItemizationRequest {
  direction: ParagraphDirection;
  /**
   * Contiguous, exhaustive, grapheme-aligned style spans. Style identities
   * must already represent effective shaping properties, not author run IDs.
   */
  spans: StyleSpan[];
  text: string;
}
/**
 * This interface was referenced by `ItemizationRequest`'s JSON-Schema
 * via the `definition` "StyleSpan".
 */
export interface StyleSpan {
  end: number;
  style: number;
}
