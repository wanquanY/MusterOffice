/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PlaybackRasterResponse =
  | {
      info: PlaybackRasterInfo;
      status: "rendered";
    }
  | {
      error: PlaybackFailure;
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
export type NavigationDirection = "next" | "previous";
export type PresentationStepOutcome =
  | {
      kind: "consumed";
    }
  | {
      entry: PresentationPageEntry;
      kind: "pageBoundary";
    };
export type PresentationPageEntry = "initial";
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
export type SlideId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type PlaybackFailure =
  | {
      error: TimelineFailure;
      kind: "timeline";
    }
  | {
      error: PageFailure;
      kind: "page";
    };
export type TimelineFailureCode =
  | "INPUT_INVALID"
  | "REVISION_CONFLICT"
  | "EVENT_HISTORY_REQUIRED"
  | "EVENT_HISTORY_INVALID"
  | "LIMIT_EXCEEDED"
  | "CANCELLED";
export type PageFailureCode =
  | "INPUT_INVALID"
  | "LIMIT_EXCEEDED"
  | "COORDINATE_RANGE"
  | "PRECISION_EXCEEDED"
  | "CANCELLED"
  | "MAPPING_NOT_IMPLEMENTED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";
export type PageFeature =
  | "table"
  | "shapeText"
  | "picture"
  | "connector"
  | "inheritedFill"
  | "inheritedStroke"
  | "unresolvedStrokeParameters"
  | "groupStroke"
  | "missingThemeColor";
export type ObjectId = string;
export type ValidationCode =
  | "IDENTITY_MISMATCH"
  | "MISSING_REFERENCE"
  | "DUPLICATE_IDENTITY"
  | "INVALID_VALUE"
  | "OWNERSHIP_CONFLICT"
  | "REFERENCE_CYCLE"
  | "LIMIT_EXCEEDED";

export interface PlaybackRasterInfo {
  frame: EvaluatedFrame;
  page: PageRasterInfo;
  profile: string;
}
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
  presentationStep?: PresentationStepReceipt | null;
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
 * The last presentation step in the sampled, validated event prefix. The
 * enclosing frame binds document/session/generation and hashes this receipt.
 */
export interface PresentationStepReceipt {
  at: ExactValue;
  direction: NavigationDirection;
  outcome: PresentationStepOutcome;
  sequence: number;
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
export interface PageRasterInfo {
  /**
   * All three stages, in raw Q32 pixels; not a raster coverage error bound.
   */
  combinedCoordinateErrorBound: string;
  page: PagePlanInfo;
  scene: SceneRasterInfo;
}
export interface PagePlanInfo {
  /**
   * Raw Q32 pixels, including author matrix quantization at control points.
   */
  authorCoordinateErrorBound: string;
  /**
   * Author percentage -> Draw IR conversion, dimensionless raw Q32.
   * Separate from device miter quantization and pixel coordinate budgets.
   */
  authorMiterLimitErrorBound: string;
  curveSegments: number;
  documentSha256: Digest;
  generatedCommands: number;
  /**
   * Raw Q32 pixels: curve approximation + control-coordinate quantization.
   */
  geometryCoordinateErrorBound: string;
  hidden: boolean;
  placementProfile: string;
  profile: string;
  slide: SlideId;
  sourceObjects: number;
}
export interface SceneRasterInfo {
  profile: string;
  raster: RasterInfo;
  work: SceneWork;
}
export interface RasterInfo {
  byteLength: ByteLength;
  /**
   * SHA-256 of the little-endian device batch; use together with profile.
   */
  frameSha256: string;
  height: number;
  profile: string;
  sha256: Digest;
  width: number;
  work: RasterWork;
}
export interface RasterWork {
  clips?: ClipWork | null;
  commands: number;
  compositing?: CompositeWork | null;
  /**
   * Conservative maximum error of the transformed control coordinates,
   * measured in raw Q32 pixels. Not a raster coverage error bound.
   */
  coordinateErrorBound: string;
  drawnCommands: number;
  draws: number;
  /**
   * Only present for V12 ellipse fields. Geometry and solver errors are separate.
   */
  ellipticGradients?: EllipticGradientWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  gradientCoordinateErrorBound: string;
  gradientDraws: number;
  gradientStops: number;
  /**
   * Maximum stop/color conversion or linear/rectangular field error.
   * Elliptic parameter and root bounds are reported separately.
   */
  gradientValueErrorBound: number;
  gradients: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  miterLimitErrorBound: string;
  opacityGroups?: OpacityGroupWork | null;
  paths: number;
  strokeDraws: number;
  strokeStyles: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  strokeWidthErrorBound: string;
}
export interface ClipWork {
  /**
   * Actual pushes when preserving common ancestors between consecutive draws.
   */
  applications: number;
  appliedCommands: number;
  maximumDepth: number;
  nodes: number;
  /**
   * All supplied clip path commands checked during device placement.
   */
  placementCommands: number;
}
export interface CompositeWork {
  /**
   * Exact packed pixel bytes copied for all snapshots; not process RSS.
   */
  capturedBytes: number;
  captures: number;
  snapshotDraws: number;
  sourceDraws: number;
}
/**
 * Maximum across elliptic fields; excludes pixel coverage and shader inverse arithmetic.
 */
export interface EllipticGradientWork {
  /**
   * Conservative device Q32 displacement of corresponding ellipse-family
   * points from parameter/plane conversion, including the tile phase budget.
   */
  coordinateErrorBound: string;
  /**
   * Q32 width of a successful solver enclosure for the encoded binary32
   * field. Not a bound on the source field's scalar or on pixel color.
   */
  encodedRootIntervalBound: string;
  /**
   * Dimensionless Q32 source uncertainty plus binary32 conversion error,
   * ordered scaleX/Y, centerX/Y, radiusX/Y.
   *
   * @minItems 6
   * @maxItems 6
   */
  parameterErrorBounds: [FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32];
}
export interface OpacityGroupWork {
  groups: number;
  maximumDepth: number;
  /**
   * Peak simultaneously live intermediate pixels, excluding output/snapshots.
   */
  peakPixelBytes: number;
  /**
   * Pixels cleared plus pixels composited, including fully transparent groups.
   */
  pixelWork: number;
}
export interface SceneWork {
  clips?: SceneClipWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  combinedCoordinateErrorBound: string;
  compiledCommands: number;
  compiledPaths: number;
  evaluatedPoints: number;
  /**
   * 2 means the shared matrix candidate failed numeric/precision checks and
   * lowering used the original node chain before any backend call.
   */
  loweringAttempts: number;
  maximumDepth: number;
  /**
   * Conservative point/node work per attempt, including identity instances.
   */
  pointTransformWork: number;
  sourceCommands: number;
  sourcePaths: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  transformErrorBound: string;
  transforms: number;
}
export interface SceneClipWork {
  compiledNodes: number;
  sourceNodes: number;
}
export interface TimelineFailure {
  code: TimelineFailureCode;
  message: string;
}
export interface PageFailure {
  code: PageFailureCode;
  feature?: PageFeature | null;
  message: string;
  object?: ObjectId | null;
  report?: ValidationReport | null;
}
export interface ValidationReport {
  issues: ValidationIssue[];
  truncated: boolean;
}
export interface ValidationIssue {
  code: ValidationCode;
  message: string;
  path: string;
}
