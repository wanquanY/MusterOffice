/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "EditContract".
 */
export type EditContract = "musteroffice.video.edit/1";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Ripple never guesses a track set. An empty scope leaves the other clips in place.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Operation".
 */
export type Operation =
  | {
      title: string;
      type: "setTitle";
    }
  | {
      sequence: Sequence;
      type: "addSequence";
    }
  | {
      before?: ObjectId | null;
      sequence: ObjectId;
      track: Track;
      type: "addTrack";
    }
  | {
      track: ObjectId;
      type: "removeTrack";
    }
  | {
      locked: boolean;
      muted: boolean;
      name: string;
      solo: boolean;
      track: ObjectId;
      type: "setTrack";
    }
  | {
      resource: Resource;
      type: "addResource";
    }
  | {
      resource: ResourceId;
      type: "removeResource";
    }
  | {
      clip: Clip;
      rippleTracks: ObjectId[];
      track: ObjectId;
      type: "insert";
    }
  | {
      clip: Clip;
      linkedPolicy: LinkedPolicy;
      remainders: SplitTarget[];
      rightGroups: LinkGroup[];
      track: ObjectId;
      type: "overwrite";
    }
  | {
      edit: RangeEdit;
      type: "deleteRange";
    }
  | {
      effects: EffectTimingPolicy;
      linkedPolicy: LinkedPolicy;
      rippleTracks: ObjectId[];
      targets: RetimeTarget[];
      type: "retime";
    }
  | {
      clips: ObjectId[];
      linkedPolicy: LinkedPolicy;
      type: "delete";
    }
  | {
      clips: ObjectId[];
      delta: RationalTime;
      linkedPolicy: LinkedPolicy;
      targetTrack?: ObjectId | null;
      type: "move";
    }
  | {
      at: RationalTime;
      linkedPolicy: LinkedPolicy;
      rightGroups: LinkGroup[];
      targets: SplitTarget[];
      type: "split";
    }
  | {
      clips: ObjectId[];
      delta: RationalTime;
      edge: Edge;
      linkedPolicy: LinkedPolicy;
      rippleTracks: ObjectId[];
      type: "trim";
    }
  | {
      clips: ObjectId[];
      delta: RationalTime;
      linkedPolicy: LinkedPolicy;
      type: "slip";
    }
  | {
      delta: RationalTime;
      left: ObjectId;
      linkedPolicy: LinkedPolicy;
      right: ObjectId;
      type: "roll";
    }
  | {
      clip: ObjectId;
      delta: RationalTime;
      left: ObjectId;
      linkedPolicy: LinkedPolicy;
      right: ObjectId;
      type: "slide";
    }
  | {
      clip: ObjectId;
      effects: Effect[];
      type: "setEffects";
    }
  | {
      clip: ObjectId;
      source: ClipSource;
      type: "replaceSource";
    }
  | {
      group: LinkGroup;
      type: "link";
    }
  | {
      group: ObjectId;
      type: "unlink";
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TrackKind".
 */
export type TrackKind = "video" | "audio" | "graphics" | "captions";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "HoldAudioPolicy".
 */
export type HoldAudioPolicy = "silence";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "AudioRatePolicy".
 */
export type AudioRatePolicy = "resample" | "preservePitch";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ReverseAudioPolicy".
 */
export type ReverseAudioPolicy = "reverse" | "mute";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * Require every member of an intersecting link group in the explicit selection, or deliberately
 * edit this instance independently. No implicit expansion or provider/Agent override.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "LinkedPolicy".
 */
export type LinkedPolicy = "requireAll" | "independent";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "EffectTimingPolicy".
 */
export type EffectTimingPolicy = "preserveClipTime" | "scaleToDuration";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Edge".
 */
export type Edge = "start" | "end";

export interface Transaction {
  contract: EditContract;
  expectedDigest: Digest;
  operations: Operation[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Sequence".
 */
export interface Sequence {
  id: ObjectId;
  tracks: ObjectId[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Resource".
 */
export interface Resource {
  byteLength: ByteLength;
  descriptor: ResourceDescriptor;
  digest: Digest;
  id: ResourceId;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Clip".
 */
export interface Clip {
  audio?: AudioTimePolicy;
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
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 */
export interface TimeRange1 {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Source-time anchors are the authority; a separate mutable speed curve is forbidden.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimeMap".
 */
export interface TimeMap {
  anchors: TimeAnchor[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimeAnchor".
 */
export interface TimeAnchor {
  local: RationalTime;
  source: RationalTime;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "SplitTarget".
 */
export interface SplitTarget {
  clip: ObjectId;
  right: ObjectId;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "LinkGroup".
 */
export interface LinkGroup {
  id: ObjectId;
  members: ObjectId[];
}
/**
 * Right-side IDs are needed exactly for the clips straddling both range boundaries.
 * The track set uses one sequence's time coordinate, never a hidden global/default scope.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RangeEdit".
 */
export interface RangeEdit {
  linkedPolicy: LinkedPolicy;
  range: TimeRange;
  remainders: SplitTarget[];
  rightGroups: LinkGroup[];
  ripple: boolean;
  tracks: ObjectId[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RetimeTarget".
 */
export interface RetimeTarget {
  audio: AudioTimePolicy1;
  clip: ObjectId;
  timeMap: TimeMap;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "AudioTimePolicy".
 */
export interface AudioTimePolicy1 {
  hold: HoldAudioPolicy;
  rate: AudioRatePolicy;
  reverse: ReverseAudioPolicy;
}
