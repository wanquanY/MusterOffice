/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "NegativeWeights".
 */
export type NegativeWeights = "reject" | "absoluteMagnitude";
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "SourceCircularProfile".
 */
export type SourceCircularProfile = "source-cache-declared-circular-plot-v1-draft";
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "ChartPlotProfile".
 */
export type ChartPlotProfile = "source-circular-declared-solid-plot-v1-draft";

export interface SourceChartPlotRequest {
  colorContext: ColorContext;
  geometry: SourceCircularRequest;
  profile: ChartPlotProfile;
}
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "ColorContext".
 */
export interface ColorContext {
  /**
   * Explicit unassociated sRGB and alpha, supplied by the owning style.
   *
   * @minItems 4
   * @maxItems 4
   */
  placeholder?: [number, number, number, number] | null;
  systemColors: {
    /**
     * @minItems 3
     * @maxItems 3
     */
    "3dDkShadow"?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    "3dLight"?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    activeBorder?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    activeCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    appWorkspace?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    background?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnFace?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnHighlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnShadow?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    captionText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    gradientActiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    gradientInactiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    grayText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    highlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    highlightText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    hotLight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveBorder?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveCaptionText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    infoBk?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    infoText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menu?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuBar?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuHighlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    scrollBar?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    window?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    windowFrame?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    windowText?: [number, number, number];
  };
}
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "SourceCircularRequest".
 */
export interface SourceCircularRequest {
  center: Point;
  coordinateTolerance: FixedQ32;
  expectedSourceSha256: Digest;
  negativeWeights: NegativeWeights;
  object: SourceObjectRef;
  outerRadius: FixedQ32;
  plotSourceOrdinal: number;
  profile: SourceCircularProfile;
}
/**
 * Resolved plot geometry, not the graphicFrame's outer box or a guessed
 * automatic layout. Labels/legend reserve space in their own layout stage.
 */
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "SourceObjectRef".
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
/**
 * This interface was referenced by `SourceChartPlotRequest`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}
