/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "HoldAudioPolicy".
 */
export type HoldAudioPolicy = "silence";
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "AudioRatePolicy".
 */
export type AudioRatePolicy = "resample" | "preservePitch";
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ReverseAudioPolicy".
 */
export type ReverseAudioPolicy = "reverse" | "mute";
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Effect".
 */
export type Effect =
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
      kind: "gain";
      milliDecibels: number;
    }
  | {
      envelope: TimeRange;
      fadeIn: RationalTime1;
      fadeOut: RationalTime1;
      kind: "fade";
    };
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ClipSource".
 */
export type ClipSource =
  | {
      kind: "media";
      resource: ResourceId;
      stream: ObjectId;
      timeMap: TimeMap;
    }
  | {
      kind: "sequence";
      sequence: ObjectId;
      timeMap: TimeMap;
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
      font: ResourceId;
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
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "VideoFormat".
 */
export type VideoFormat = "musteroffice.video/1";
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ResourceDescriptor".
 */
export type ResourceDescriptor =
  | {
      kind: "media";
      streams: {
        [k: string]: MediaStream | undefined;
      };
    }
  | {
      faceIndex: number;
      kind: "font";
    };
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "MediaStream".
 */
export type MediaStream =
  | {
      available: TimeRange1;
      dimensions: Canvas;
      frameRate?: FrameRate | null;
      kind: "video";
    }
  | {
      available: TimeRange1;
      channels: number;
      kind: "audio";
      sampleRate: number;
    }
  | {
      dimensions: Canvas;
      kind: "image";
    };
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "TrackKind".
 */
export type TrackKind = "video" | "audio" | "graphics" | "captions";

/**
 * Content only: resource bytes, URLs, project head, jobs, permissions and caches live in the host.
 */
export interface VideoDocument {
  audioSampleRate: number;
  canvas: Canvas;
  clips: ObjectMap3;
  format: VideoFormat;
  frameRate: FrameRate;
  id: DocumentId;
  linkGroups: ObjectMap5;
  resources: ObjectMap4;
  rootSequence: ObjectId;
  sequences: ObjectMap;
  title: string;
  tracks: ObjectMap2;
}
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ObjectMap3".
 */
export interface ObjectMap3 {
  [k: string]: Clip | undefined;
}
/**
 * This interface was referenced by `ObjectMap3`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Clip".
 */
export interface Clip {
  audio?: AudioTimePolicy;
  /**
   * The complete pre-trim processing map. Visible cuts are views of this
   * continuous DSP context, so cutting a clip does not restart its vocoder.
   */
  audioContext?: AudioRenderContext | null;
  effects: Effect[];
  id: ObjectId;
  name: string;
  source: ClipSource;
  timeline: TimeRange1;
}
/**
 * Explicitly persisted even for silent visual clips; only an audio-track compiler consumes it.
 */
export interface AudioTimePolicy {
  hold: HoldAudioPolicy;
  rate: AudioRatePolicy;
  reverse: ReverseAudioPolicy;
}
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "AudioRenderContext".
 */
export interface AudioRenderContext {
  offset: RationalTime;
  timeMap: TimeMap;
}
/**
 * Clip-local zero in context-local time. The source binding remains the
 * clip's binding; this cannot reference a different resource or sequence.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Source-time anchors are the authority; a separate mutable speed curve is forbidden.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "TimeMap".
 */
export interface TimeMap {
  anchors: TimeAnchor[];
}
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "TimeAnchor".
 */
export interface TimeAnchor {
  local: RationalTime1;
  source: RationalTime1;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime1 {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Fade phase in clip-local time; splitting shifts this range without restarting it.
 */
export interface TimeRange {
  end: RationalTime1;
  start: RationalTime1;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange1 {
  end: RationalTime1;
  start: RationalTime1;
}
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ObjectMap5".
 */
export interface ObjectMap5 {
  [k: string]: LinkGroup | undefined;
}
/**
 * This interface was referenced by `ObjectMap5`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "LinkGroup".
 */
export interface LinkGroup {
  id: ObjectId;
  members: ObjectId[];
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ObjectMap4".
 */
export interface ObjectMap4 {
  [k: string]: Resource | undefined;
}
/**
 * This interface was referenced by `ObjectMap4`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Resource".
 */
export interface Resource {
  byteLength: ByteLength;
  descriptor: ResourceDescriptor;
  digest: Digest;
  id: ResourceId;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ObjectMap".
 */
export interface ObjectMap {
  [k: string]: Sequence | undefined;
}
/**
 * This interface was referenced by `ObjectMap`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Sequence".
 */
export interface Sequence {
  id: ObjectId;
  tracks: ObjectId[];
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "ObjectMap2".
 */
export interface ObjectMap2 {
  [k: string]: Track | undefined;
}
/**
 * This interface was referenced by `ObjectMap2`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "Track".
 */
export interface Track {
  clips: ObjectId[];
  id: ObjectId;
  kind: TrackKind;
  locked: boolean;
  muted: boolean;
  name: string;
  solo: boolean;
}
/**
 * This interface was referenced by `VideoDocument`'s JSON-Schema
 * via the `definition` "AudioTimePolicy".
 */
export interface AudioTimePolicy1 {
  hold: HoldAudioPolicy;
  rate: AudioRatePolicy;
  reverse: ReverseAudioPolicy;
}
