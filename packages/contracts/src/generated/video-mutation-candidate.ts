/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectKey".
 */
export type ObjectKey =
  | {
      kind: "header";
    }
  | {
      id: ObjectId;
      kind: "sequence";
    }
  | {
      id: ObjectId;
      kind: "track";
    }
  | {
      id: ObjectId;
      kind: "clip";
    }
  | {
      id: ResourceId;
      kind: "resource";
    }
  | {
      id: ObjectId;
      kind: "linkGroup";
    };
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "RestoreObject".
 */
export type RestoreObject =
  | {
      kind: "title";
      title: string;
    }
  | {
      id: ObjectId;
      kind: "sequence";
      value?: Sequence | null;
    }
  | {
      id: ObjectId;
      kind: "track";
      value?: Track | null;
    }
  | {
      id: ObjectId;
      kind: "clip";
      value?: Clip | null;
    }
  | {
      id: ResourceId;
      kind: "resource";
      value?: Resource | null;
    }
  | {
      id: ObjectId;
      kind: "linkGroup";
      value?: LinkGroup | null;
    };
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "TrackKind".
 */
export type TrackKind = "video" | "audio" | "graphics" | "captions";
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "HoldAudioPolicy".
 */
export type HoldAudioPolicy = "silence";
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "AudioRatePolicy".
 */
export type AudioRatePolicy = "resample" | "preservePitch";
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ReverseAudioPolicy".
 */
export type ReverseAudioPolicy = "reverse" | "mute";
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
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
      envelope: TimeRange1;
      fadeIn: RationalTime;
      fadeOut: RationalTime;
      kind: "fade";
    };
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
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
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
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
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
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
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "VideoFormat".
 */
export type VideoFormat = "musteroffice.video/1";
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "SnapshotFormat".
 */
export type SnapshotFormat = "musteroffice.video.snapshot/1";

/**
 * This is a computation candidate, never a durable host commit receipt.
 */
export interface MutationCandidate {
  baseDigest: Digest;
  invalidations: Invalidation[];
  inverse: Inverse;
  readSet: ObjectVersion[];
  requestDigest: Digest;
  snapshot: VideoSnapshot;
  writeSet: ObjectKey[];
}
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Invalidation".
 */
export interface Invalidation {
  ranges: TimeRange[];
  track: ObjectId;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * A conditional inverse, not a whole-project rewind. The host binds this to its authorized
 * history record; the core neither grants permission nor makes a durable commit.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Inverse".
 */
export interface Inverse {
  conditions: ObjectVersion[];
  restore: RestoreObject[];
}
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectVersion".
 */
export interface ObjectVersion {
  digest: Digest;
  object: ObjectKey;
}
/**
 * This interface was referenced by `ObjectMap`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Sequence".
 */
export interface Sequence {
  id: ObjectId;
  tracks: ObjectId[];
}
/**
 * This interface was referenced by `ObjectMap2`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
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
 * This interface was referenced by `ObjectMap3`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
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
  timeline: TimeRange;
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
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "AudioRenderContext".
 */
export interface AudioRenderContext {
  offset: RationalTime1;
  timeMap: TimeMap;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime1 {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Source-time anchors are the authority; a separate mutable speed curve is forbidden.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "TimeMap".
 */
export interface TimeMap {
  anchors: TimeAnchor[];
}
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "TimeAnchor".
 */
export interface TimeAnchor {
  local: RationalTime;
  source: RationalTime;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 */
export interface TimeRange1 {
  end: RationalTime;
  start: RationalTime;
}
/**
 * This interface was referenced by `ObjectMap4`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Resource".
 */
export interface Resource {
  byteLength: ByteLength;
  descriptor: ResourceDescriptor;
  digest: Digest;
  id: ResourceId;
}
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
/**
 * This interface was referenced by `ObjectMap5`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "LinkGroup".
 */
export interface LinkGroup {
  id: ObjectId;
  members: ObjectId[];
}
/**
 * Portable authoring snapshot. No host revision, permissions, paths or jobs.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "VideoSnapshot".
 */
export interface VideoSnapshot {
  document: VideoDocument;
  documentDigest: Digest;
  format: SnapshotFormat;
}
/**
 * Content only: resource bytes, URLs, project head, jobs, permissions and caches live in the host.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "VideoDocument".
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
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectMap3".
 */
export interface ObjectMap3 {
  [k: string]: Clip | undefined;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectMap5".
 */
export interface ObjectMap5 {
  [k: string]: LinkGroup | undefined;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectMap4".
 */
export interface ObjectMap4 {
  [k: string]: Resource | undefined;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectMap".
 */
export interface ObjectMap {
  [k: string]: Sequence | undefined;
}
/**
 * Ordered persistent object storage: cloning shares the tree, edits copy only the search path
 * and the edited record. Wire data and schema remain ordinary JSON maps.
 *
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "ObjectMap2".
 */
export interface ObjectMap2 {
  [k: string]: Track | undefined;
}
/**
 * This interface was referenced by `MutationCandidate`'s JSON-Schema
 * via the `definition` "AudioTimePolicy".
 */
export interface AudioTimePolicy1 {
  hold: HoldAudioPolicy;
  rate: AudioRatePolicy;
  reverse: ReverseAudioPolicy;
}
