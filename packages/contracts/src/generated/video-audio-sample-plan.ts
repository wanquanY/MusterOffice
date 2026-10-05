/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * A one-sided sample at an exact instant. Reversal flips the side rather than inventing an epsilon.
 *
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "SampleSide".
 */
export type SampleSide = "after" | "before";
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "AudioEffect".
 */
export type AudioEffect =
  | {
      kind: "gain";
      milliDecibels: number;
    }
  | {
      factor: ExactRatio;
      kind: "amplitude";
    };
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "HoldAudioPolicy".
 */
export type HoldAudioPolicy = "silence";
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "AudioRatePolicy".
 */
export type AudioRatePolicy = "resample" | "preservePitch";
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "ReverseAudioPolicy".
 */
export type ReverseAudioPolicy = "reverse" | "mute";
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "AudioSource".
 */
export type AudioSource =
  | {
      binding: MediaBinding;
      kind: "media";
    }
  | {
      kind: "group";
      sequence: ObjectId;
      voices: AudioNode[];
    };
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
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
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

/**
 * Point inspection/oracle, not a PCM block-processing API. Production DSP must schedule spans.
 */
export interface AudioSamplePlan {
  documentDigest: Digest;
  sample: TimeSample;
  sampleIndex: Ticks;
  sampleRate: number;
  voices: AudioNode[];
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "TimeSample".
 */
export interface TimeSample {
  side: SampleSide;
  time: ExactRatio;
}
/**
 * Exact computed fraction. Decimal strings are bounded to 4096 bits and canonicalized by the kernel; each computation can impose a smaller budget.
 *
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "ExactRatio".
 */
export interface ExactRatio {
  denominator: string;
  numerator: string;
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "AudioNode".
 */
export interface AudioNode {
  clip: ObjectId;
  effects: AudioEffect[];
  local: TimeSample;
  mapping: MappedTime;
  policy: AudioTimePolicy;
  source: AudioSource;
  track: ObjectId;
}
/**
 * Each hierarchy stage retains its own rate/pitch policy and gain; do not flatten DSP stages.
 */
export interface MappedTime {
  rate: ExactRatio;
  sample: TimeSample;
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "AudioTimePolicy".
 */
export interface AudioTimePolicy {
  hold: HoldAudioPolicy;
  rate: AudioRatePolicy;
  reverse: ReverseAudioPolicy;
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
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
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
/**
 * This interface was referenced by `AudioSamplePlan`'s JSON-Schema
 * via the `definition` "MappedTime".
 */
export interface MappedTime1 {
  rate: ExactRatio;
  sample: TimeSample;
}
