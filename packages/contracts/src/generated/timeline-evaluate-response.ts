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
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
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
  nodes: NodeFrame[];
  profile: string;
  rotations: {
    [k: string]: ExactValue | undefined;
  };
  time: RationalTime;
  timelineSha256: Digest;
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
 * Output-only exact reduced ratio. Rotation units remain 1/60000 degree.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactValue {
  denominator: string;
  numerator: string;
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
