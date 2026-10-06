/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "AudioSpanEffect".
 */
export type AudioSpanEffect =
  | {
      kind: "gain";
      milliDecibels: number;
    }
  | {
      envelope: TimeRange;
      fadeIn: RationalTime;
      fadeOut: RationalTime;
      kind: "fade";
    };
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * A one-sided sample at an exact instant. Reversal flips the side rather than inventing an epsilon.
 *
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "SampleSide".
 */
export type SampleSide = "after" | "before";
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "HoldAudioPolicy".
 */
export type HoldAudioPolicy = "silence";
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "AudioRatePolicy".
 */
export type AudioRatePolicy = "resample" | "preservePitch";
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "ReverseAudioPolicy".
 */
export type ReverseAudioPolicy = "reverse" | "mute";
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "AudioSpanSource".
 */
export type AudioSpanSource =
  | {
      binding: MediaBinding;
      kind: "media";
    }
  | {
      kind: "sequence";
      sequence: ObjectId;
    };
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
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
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

export interface AudioBlockPlan {
  documentDigest: Digest;
  firstSample: Ticks;
  frames: number;
  sampleRate: number;
  sequence: ObjectId;
  voices: AudioSpanVoice[];
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "AudioSpanVoice".
 */
export interface AudioSpanVoice {
  clip: ObjectId;
  effects: AudioSpanEffect[];
  /**
   * Signed int64 ticks. Range requires semantic validation.
   */
  firstSample: string;
  frames: number;
  localStart: ExactRatio;
  mapping: MappedTime;
  policy: AudioTimePolicy;
  source: AudioSpanSource;
  track: ObjectId;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Exact computed fraction. Decimal strings are bounded to 4096 bits and canonicalized by the kernel; each computation can impose a smaller budget.
 *
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "ExactRatio".
 */
export interface ExactRatio {
  denominator: string;
  numerator: string;
}
/**
 * At first_sample. Rate is constant until the next authored time-map anchor.
 */
export interface MappedTime {
  rate: ExactRatio;
  sample: TimeSample;
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "TimeSample".
 */
export interface TimeSample {
  side: SampleSide;
  time: ExactRatio;
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "AudioTimePolicy".
 */
export interface AudioTimePolicy {
  hold: HoldAudioPolicy;
  rate: AudioRatePolicy;
  reverse: ReverseAudioPolicy;
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "MediaBinding".
 */
export interface MediaBinding {
  descriptor: MediaStream;
  digest: Digest;
  resource: ResourceId;
  stream: ObjectId;
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
/**
 * This interface was referenced by `AudioBlockPlan`'s JSON-Schema
 * via the `definition` "MappedTime".
 */
export interface MappedTime1 {
  rate: ExactRatio;
  sample: TimeSample;
}
