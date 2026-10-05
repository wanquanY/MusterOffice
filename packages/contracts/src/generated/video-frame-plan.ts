/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "VisualEffect".
 */
export type VisualEffect =
  | {
      kind: "transform";
      opacityPpm: number;
      rotationMilliDegrees: number;
      scaleXPpm: number;
      scaleYPpm: number;
      xMilliPixels: number;
      yMilliPixels: number;
    }
  | {
      factor: ExactRatio;
      kind: "opacity";
    };
/**
 * A one-sided sample at an exact instant. Reversal flips the side rather than inventing an epsilon.
 *
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "SampleSide".
 */
export type SampleSide = "after" | "before";
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "VisualSource".
 */
export type VisualSource =
  | {
      binding: MediaBinding;
      kind: "media";
      mapping: MappedTime;
    }
  | {
      kind: "group";
      layers: VisualNode[];
      mapping: MappedTime;
      sequence: ObjectId;
    }
  | {
      kind: "solid";
      /**
       * @minItems 4
       * @maxItems 4
       */
      rgba: [number, number, number, number];
    }
  | {
      faceIndex: number;
      font: ResourceId;
      fontDigest: Digest;
      fontSizeMilliPixels: number;
      kind: "text";
      /**
       * @minItems 4
       * @maxItems 4
       */
      rgba: [number, number, number, number];
      text: string;
    };
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "MediaStream".
 */
export type MediaStream =
  | {
      available: TimeRange;
      dimensions: Canvas;
      frameRate?: FrameRate | null;
      kind: "video";
    }
  | {
      available: TimeRange;
      channels: number;
      kind: "audio";
      sampleRate: number;
    }
  | {
      dimensions: Canvas;
      kind: "image";
    };
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

export interface VideoFramePlan {
  canvas: Canvas;
  documentDigest: Digest;
  frameIndex: Ticks;
  /**
   * Bottom-to-top compositing order. Nested groups must be composited before group effects.
   */
  layers: VisualNode[];
  sample: TimeSample;
}
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "VisualNode".
 */
export interface VisualNode {
  clip: ObjectId;
  /**
   * Ordered effects; group opacity is NOT distributive over child layers.
   */
  effects: VisualEffect[];
  local: TimeSample;
  source: VisualSource;
  track: ObjectId;
}
/**
 * Exact computed fraction. Decimal strings are bounded to 4096 bits and canonicalized by the kernel; each computation can impose a smaller budget.
 *
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "ExactRatio".
 */
export interface ExactRatio {
  denominator: string;
  numerator: string;
}
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "TimeSample".
 */
export interface TimeSample {
  side: SampleSide;
  time: ExactRatio;
}
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "MediaBinding".
 */
export interface MediaBinding {
  descriptor: MediaStream;
  digest: Digest;
  resource: ResourceId;
  stream: ObjectId;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
/**
 * This interface was referenced by `VideoFramePlan`'s JSON-Schema
 * via the `definition` "MappedTime".
 */
export interface MappedTime {
  rate: ExactRatio;
  sample: TimeSample;
}
