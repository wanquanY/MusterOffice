/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxTimingResponse =
  | {
      status: "inspected";
      timing: SourceTiming;
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type TimelineVersion = "musteroffice.timeline/0.1-draft" | "musteroffice.timeline/0.2-draft";
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
export type Effect =
  | {
      composition?: RotationComposition;
      from: number;
      kind: "rotation";
      target: ObjectId;
      to: number;
    }
  | {
      from: ScaleValue;
      kind: "scale";
      target: ObjectId;
      to: ScaleValue;
    }
  | {
      kind: "setVisibility";
      target: ObjectId;
      value: Visibility;
    }
  | {
      from: MotionPoint;
      kind: "motionLine";
      target: ObjectId;
      to: MotionPoint;
    }
  | {
      kind: "motionPath";
      path: MotionPath;
      target: ObjectId;
    }
  | {
      kind: "fade";
      target: ObjectId;
      transition: FadeTransition;
    };
/**
 * Rotation is composed before object/group placement. Layout replaces earlier
 * animation offsets while preserving the document's local orientation. Add
 * sums the sampled offset with the lower-priority visible rotation stack.
 */
export type RotationComposition = "absolute" | "layout" | "add";
export type ObjectId = string;
export type Visibility = "visible" | "hidden";
/**
 * Exact decimal fraction of the slide dimension; canonicalized without rounding.
 */
export type MotionCoordinate = string;
/**
 * Source control points remain editable; subdivision belongs only to the
 * immutable playback plan. Close returns to the initial `from` point.
 */
export type MotionSegment =
  | {
      kind: "line";
      to: MotionPoint;
    }
  | {
      control1: MotionPoint;
      control2: MotionPoint;
      kind: "cubic";
      to: MotionPoint;
    }
  | {
      kind: "close";
    };
export type FadeTransition = "in" | "out";
export type TimeCondition =
  | {
      kind: "never";
    }
  | {
      kind: "at";
      offset: RationalTime;
    }
  | {
      delay: RationalTime;
      event: NodeEvent;
      kind: "after";
      node: TimingNodeId;
    }
  | {
      delay: RationalTime;
      kind: "click";
      target?: ObjectId | null;
    }
  | {
      delay: RationalTime;
      direction: NavigationDirection;
      kind: "navigation";
      target?: ObjectId | null;
    };
export type NodeEvent = ("end" | "onEnd") | "begin" | "onBegin";
export type TimingNodeId = string;
export type NavigationDirection = "next" | "previous";
export type FillMode = ("remove" | "freeze") | "hold";
export type RepeatDuration = "indefinite" | RationalTime;
/**
 * Admission of new begin instances within one parent activation. Ancestor
 * reactivation resets this policy, including `Never`. Omission preserves the
 * existing draft's once-per-parent behavior, independently of native defaults.
 */
export type RestartMode = "never" | "always" | "whenNotActive";
/**
 * A flat disjunction of native begin conditions. The single-condition wire
 * representation remains unchanged; alternatives cannot recursively nest.
 */
export type StartCondition =
  | {
      /**
       * @minItems 1
       */
      conditions: [TimeCondition, ...TimeCondition[]];
      kind: "anyOf";
    }
  | TimeCondition;
export type ContainerDuration =
  | {
      kind: "automatic";
    }
  | {
      duration: RationalTime;
      kind: "fixed";
    }
  | {
      kind: "indefinite";
    };
export type ContainerKind = "parallel" | "sequence";
export type NextAction = "none" | "seek";
export type PreviousAction = "none" | "skipTimed";
export type PresentationRole =
  | {
      kind: "mainSequence";
    }
  | {
      kind: "effect";
      preset: PresentationPreset;
      trigger: PresentationTrigger;
    };
export type PresentationPreset = "appear" | "disappear" | "spin" | "growShrink" | "customMotion" | "fadeIn" | "fadeOut";
export type PresentationTrigger = "click" | "withPrevious" | "afterPrevious";
export type Digest = string;
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceTiming {
  native?: NativeTimeline | null;
  partSha256: Digest;
  slide: string;
  sourceSha256: Digest;
}
export interface NativeTimeline {
  nodeBindings: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: number | undefined;
  };
  objectBindings: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: number | undefined;
  };
  rootId: number;
  timeline: Timeline;
}
/**
 * Behaviors plus an optional explicit timing forest. The legacy graph retains
 * its byte representation; version 0.2 owns every behavior through tree roots.
 */
export interface Timeline {
  format: TimelineVersion;
  nodes: TimingNode[];
  tree?: TimingTree | null;
}
export interface TimingNode {
  duration: RationalTime;
  effect: Effect;
  /**
   * Earliest resolved eligible end; absent conditions add no end constraint.
   */
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  /**
   * Additional bound in local active time, before speed scaling.
   */
  repeatDuration?: RepeatDuration | null;
  /**
   * Native count in thousandths, or explicit indefinite repetition.
   */
  repeatMilli: "indefinite" | number;
  restart?: RestartMode;
  start: StartCondition;
  timeTransform?: TimeTransform | null;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
export interface ScaleValue {
  x: number;
  y: number;
}
export interface MotionPoint {
  x: MotionCoordinate;
  y: MotionCoordinate;
}
/**
 * Connected native path. Coordinates are absolute offsets from the original
 * layout center, measured in slide fractions. Pacing uses length in this
 * normalized coordinate space, before scaling the axes to slide dimensions.
 */
export interface MotionPath {
  from: MotionPoint;
  segments: MotionSegment[];
}
/**
 * Local behavior clock. Percentages use native thousandths of one percent;
 * 100000 speed is normal playback. The clock is independent of effect values.
 */
export interface TimeTransform {
  accelerationMilliPercent: number;
  autoReverse: boolean;
  decelerationMilliPercent: number;
  speedMilliPercent: number;
}
export interface TimingTree {
  containers: TimingContainer[];
  roots: TimingNodeId[];
}
export interface TimingContainer {
  children: TimingNodeId[];
  duration: ContainerDuration;
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  kind: ContainerKind;
  navigation?: SequenceNavigation | null;
  /**
   * Native presentation identity. It participates in initial playback state
   * and editable export; it never changes the container's declared clock.
   */
  presentation?: PresentationRole | null;
  restart?: RestartMode;
  start: StartCondition;
  /**
   * Filter the container's simple time before its descendants consume it.
   * Compilation validates the supported clock domain; this is never copied
   * into the leaves or interpreted as an independent per-effect easing.
   */
  timeTransform?: TimeTransform | null;
}
/**
 * Sequence controls are document computation, independent of host buttons or
 * keyboard bindings. Conditions are disjunctions, like begin/end conditions.
 */
export interface SequenceNavigation {
  concurrent: boolean;
  nextAction: NextAction;
  nextConditions: TimeCondition[];
  previousAction: PreviousAction;
  previousConditions: TimeCondition[];
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
