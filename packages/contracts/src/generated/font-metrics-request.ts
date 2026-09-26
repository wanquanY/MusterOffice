/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `FontMetricsRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `FontMetricsRequest`'s JSON-Schema
 * via the `definition` "FontMetric".
 */
export type FontMetric =
  | "horizontalAscender"
  | "horizontalDescender"
  | "horizontalLineGap"
  | "horizontalClippingAscent"
  | "horizontalClippingDescent"
  | "verticalAscender"
  | "verticalDescender"
  | "verticalLineGap"
  | "horizontalCaretRise"
  | "horizontalCaretRun"
  | "horizontalCaretOffset"
  | "verticalCaretRise"
  | "verticalCaretRun"
  | "verticalCaretOffset"
  | "xHeight"
  | "capHeight"
  | "subscriptXSize"
  | "subscriptYSize"
  | "subscriptXOffset"
  | "subscriptYOffset"
  | "superscriptXSize"
  | "superscriptYSize"
  | "superscriptXOffset"
  | "superscriptYOffset"
  | "strikeoutSize"
  | "strikeoutOffset"
  | "underlineSize"
  | "underlineOffset";

export interface FontMetricsRequest {
  expectedSha256: Digest;
  faceIndex: number;
  instances: FontMetricsInstance[];
}
/**
 * This interface was referenced by `FontMetricsRequest`'s JSON-Schema
 * via the `definition` "FontMetricsInstance".
 */
export interface FontMetricsInstance {
  metrics: FontMetric[];
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `FontMetricsRequest`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
