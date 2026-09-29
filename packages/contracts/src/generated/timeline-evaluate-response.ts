/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type TimelineEvaluateResponse =
  | {
      frame: EvaluatedFrame;
      status: "evaluated";
    }
  | {
      error: TimelineFailure;
      status: "error";
    };
export type Digest = string;
/**
 * Canonical uint64 playback generation; never wraps.
 */
export type PlaybackGeneration = string;
export type PlaybackSessionId = string;
export type TimingNodeId = string;
export type NodePhase = "waiting" | "scheduled" | "active" | "frozen" | "finished" | "suppressed";
export type RotationBasis = "absolute" | "layout";
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export type Visibility = "visible" | "hidden";
export type TimelineFailureCode =
  | "INPUT_INVALID"
  | "REVISION_CONFLICT"
  | "EVENT_HISTORY_REQUIRED"
  | "EVENT_HISTORY_INVALID"
  | "LIMIT_EXCEEDED"
  | "CANCELLED";

export interface EvaluatedFrame {
  sha256: Digest;
  state: FrameState;
}
export interface FrameState {
  binding: PlaybackBinding;
  containers?: NodeFrame[];
  eventCursor: number;
  /**
   * Exact slide-relative offsets from the original layout center.
   */
  motion?: {
    [k: string]: ExactMotion | undefined;
  };
  nodes: NodeFrame[];
  /**
   * Exact whole-object opacity in [0, 1], before one render-boundary rounding.
   */
  opacity?: {
    [k: string]: ExactValue | undefined;
  };
  profile: string;
  rotations: {
    [k: string]: ExactRotation | undefined;
  };
  scales?: {
    [k: string]: ExactScale | undefined;
  };
  sequences?: SequenceFrame[];
  time: RationalTime;
  timelineSha256: Digest;
  visibility?: {
    [k: string]: Visibility | undefined;
  };
}
export interface PlaybackBinding {
  generation: PlaybackGeneration;
  revision: Digest;
  session: PlaybackSessionId;
}
export interface NodeFrame {
  end?: ExactValue | null;
  iteration?: string | null;
  node: TimingNodeId;
  phase: NodePhase;
  progress?: ExactValue | null;
  start?: ExactValue | null;
}
/**
 * Output-only exact reduced ratio. Units are defined by each property channel.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactValue {
  denominator: string;
  numerator: string;
}
/**
 * Position offsets in fractions of the slide width/height. Ratios encode the
 * computed position exactly; the frame profile states any path approximation.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactMotion {
  x: ExactValue;
  y: ExactValue;
}
/**
 * An exact angle with its document dependency still explicit. The layout
 * orientation is resolved only at placement, after source inheritance.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactRotation {
  basis?: RotationBasis;
  denominator: string;
  numerator: string;
}
/**
 * Scale values remain thousandths of a percent until the placement boundary.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactScale {
  x: ExactValue;
  y: ExactValue;
}
export interface SequenceFrame {
  current?: TimingNodeId | null;
  node: TimingNodeId;
  /**
   * Zero-based cursor. The child count denotes the position after the end.
   */
  position: number;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
export interface TimelineFailure {
  code: TimelineFailureCode;
  message: string;
}
