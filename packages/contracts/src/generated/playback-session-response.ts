/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PlaybackSessionResponse =
  | {
      info: PlaybackSessionInfo;
      status: "prepared";
    }
  | {
      info: PlaybackSessionInfo;
      status: "inspected";
    }
  | {
      info: PlaybackTimingInfo;
      status: "timingInspected";
    }
  | {
      info: PlaybackSessionInfo;
      status: "advanced";
    }
  | {
      binding: PlaybackBinding;
      status: "disposed";
    }
  | {
      frame: PlaybackCompiledFrame;
      status: "compiled";
    }
  | {
      info: PlaybackRasterInfo;
      status: "rendered";
    }
  | {
      error: PlaybackSessionFailure;
      status: "error";
    };
/**
 * Canonical uint64 playback generation; never wraps.
 */
export type PlaybackGeneration = string;
export type Digest = string;
export type PlaybackSessionId = string;
export type SlideId = string;
/**
 * Canonical uint64 timeline work count; never wraps.
 */
export type TimelineWorkCount = string;
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
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type ContainerId =
  | {
      id: SlideId;
      kind: "slide";
    }
  | {
      id: MasterId;
      kind: "master";
    }
  | {
      id: LayoutId;
      kind: "layout";
    }
  | {
      id: ObjectId;
      kind: "group";
    };
export type MasterId = string;
export type LayoutId = string;
export type ObjectId = string;
export type PagePaintKind = "fill" | "stroke";
/**
 * Porter-Duff composition, separate from the source brush and geometry mask.
 */
export type BlendMode = "sourceOver" | "source";
export type GradientAlpha = "straight" | "premultiplied";
export type GradientGeometry =
  | {
      end: Point;
      kind: "linear";
      start: Point;
    }
  | {
      center: Point;
      kind: "radial";
      radius: FixedQ32;
    }
  | {
      field: GradientField;
      kind: "plane";
      plane: GradientPlane;
    };
export type GradientField =
  | {
      /**
       * @minItems 2
       * @maxItems 2
       */
      innerCenter: [FixedQ32, FixedQ32];
      /**
       * @minItems 2
       * @maxItems 2
       */
      innerRadii: [FixedQ32, FixedQ32];
      kind: "elliptic";
      /**
       * Each dimensionless scale is in (0,1].
       *
       * @minItems 2
       * @maxItems 2
       */
      tileScale: [FixedQ32, FixedQ32];
      /**
       * Nonnegative Q32 errors ordered scaleX/Y, centerX/Y, radiusX/Y.
       * They bound geometry parameters, not a uniform scalar/color error.
       *
       * @minItems 6
       * @maxItems 6
       */
      uncertainty?: [FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32] | null;
    }
  | {
      /**
       * @minItems 3
       * @maxItems 3
       */
      coefficients: [FixedQ32, FixedQ32, FixedQ32];
      kind: "linear";
      /**
       * Nonnegative dimensionless Q32 errors, before float conversion.
       *
       * @minItems 3
       * @maxItems 3
       */
      uncertainty?: [FixedQ32, FixedQ32, FixedQ32] | null;
    }
  | {
      /**
       * @minItems 4
       * @maxItems 4
       */
      edgeRates: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
      kind: "rectangular";
      /**
       * @minItems 4
       * @maxItems 4
       */
      uncertainty?: [FixedQ32, FixedQ32, FixedQ32, FixedQ32] | null;
    };
export type GradientAxisTile = "clamp" | "repeat" | "mirror";
export type GradientInterpolation = ("srgb" | "linearSrgb") | "officeGamma1875";
export type GradientTile = "clamp" | "repeat" | "mirror" | "decal";
export type ImageSampling = "nearest" | "linear";
export type ImageTile = "clamp" | "repeat" | "mirror" | "decal";
/**
 * The source canvas at a capture point. Explicit captures may be consumed
 * later in any group; their pixels remain immutable after the source closes.
 */
export type SnapshotScope =
  | {
      kind: "current";
    }
  | {
      kind: "output";
    }
  | {
      index: number;
      kind: "group";
    };
export type StrokeCap = "butt" | "round" | "square";
export type StrokeJoin =
  | {
      kind: "miter";
      limit: FixedQ32;
    }
  | {
      kind: "miterClip";
      limit: FixedQ32;
    }
  | {
      kind: "round";
    }
  | {
      kind: "bevel";
    };
export type PathCommand =
  | {
      kind: "move";
      to: Point;
    }
  | {
      kind: "line";
      to: Point;
    }
  | {
      control: Point;
      kind: "quadratic";
      to: Point;
    }
  | {
      control1: Point;
      control2: Point;
      kind: "cubic";
      to: Point;
    }
  | {
      kind: "close";
    };
export type FillRule = "nonzero" | "evenodd";
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type PlaybackSessionFailure =
  | {
      code: PlaybackSessionFailureCode;
      kind: "session";
      message: string;
    }
  | {
      error: PlaybackFailure;
      kind: "computation";
    };
export type PlaybackSessionFailureCode =
  | "INPUT_INVALID"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "NOT_PREPARED"
  | "ALREADY_PREPARED"
  | "DISPOSED"
  | "BINDING_CONFLICT"
  | "GENERATION_NOT_INCREASING"
  | "RASTER_REQUIRED";
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
export type ValidationCode =
  | "IDENTITY_MISMATCH"
  | "MISSING_REFERENCE"
  | "DUPLICATE_IDENTITY"
  | "INVALID_VALUE"
  | "OWNERSHIP_CONFLICT"
  | "REFERENCE_CYCLE"
  | "LIMIT_EXCEEDED";

export interface PlaybackSessionInfo {
  binding: PlaybackBinding;
  documentSha256: Digest;
  /**
   * Content identity for this implementation profile, not an authority token
   * or a portable serialized-plan compatibility promise.
   */
  planId: string;
  profile: string;
  slide: SlideId;
}
export interface PlaybackBinding {
  generation: PlaybackGeneration;
  revision: Digest;
  session: PlaybackSessionId;
}
/**
 * Read-only diagnostics. Counts successful timing evaluations, including those
 * followed by a page/raster failure; does not count published or displayed frames.
 */
export interface PlaybackTimingInfo {
  binding: PlaybackBinding;
  sampler: TimelineSamplerInfo;
}
export interface TimelineSamplerInfo {
  cachedBinding?: PlaybackBinding | null;
  retainedEvents: TimelineWorkCount;
  retainedIntervals: TimelineWorkCount;
  /**
   * Canonical uint64 timeline work count; never wraps.
   */
  schedulesBuilt: string;
  schedulesReused: TimelineWorkCount;
  timelineSha256: Digest;
}
export interface PlaybackCompiledFrame {
  frame: EvaluatedFrame;
  page: PagePlan;
  placements: PagePlacements;
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
export interface PagePlan {
  combinedCoordinateErrorBound: FixedQ32;
  deviceWork: SceneWork;
  info: PagePlanInfo;
  paintSources: PagePaintSource[];
  raster: SceneRasterRequest;
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
export interface PagePlanInfo {
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  authorCoordinateErrorBound: string;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  authorMiterLimitErrorBound: string;
  curveSegments: number;
  documentSha256: Digest;
  generatedCommands: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  geometryCoordinateErrorBound: string;
  hidden: boolean;
  placementProfile: string;
  profile: string;
  slide: SlideId;
  sourceObjects: number;
}
export interface PagePaintSource {
  container: ContainerId;
  instance: number;
  /**
   * None denotes the page background. Groups create spaces, not paint draws.
   */
  object?: ObjectId | null;
  paint: PagePaintKind;
}
/**
 * Reduced device tolerance reserves the explicit author/geometry budgets.
 */
export interface SceneRasterRequest {
  scene: DrawScene;
  viewport: RasterViewport;
}
/**
 * Evaluated resources/instances, independent of authoring document semantics.
 */
export interface DrawScene {
  clips?: ClipNode[];
  instances: PathInstance[];
  /**
   * Intervals refer to instances; lowering preserves their order and count.
   */
  opacityGroups?: OpacityGroup[];
  paths: FillPath[];
  transforms: TransformNode[];
}
/**
 * Evaluated intersection path; geometry has its own transform. Parent clipping
 * remains in world coordinates and is never transformed by a child or draw.
 */
export interface ClipNode {
  parent?: number | null;
  path: number;
  transform?: number | null;
}
export interface PathInstance {
  blend?: BlendMode;
  /**
   * Already evaluated world-space paint, independent of this path transform.
   */
  brush:
    | {
        kind: "solid";
        /**
         * @minItems 4
         * @maxItems 4
         */
        rgba: [number, number, number, number];
      }
    | {
        gradient: Gradient;
        kind: "gradient";
      }
    | {
        image: ImageBrush;
        kind: "image";
      }
    | {
        afterDraws: number;
        kind: "snapshot";
        scope?: SnapshotScope;
      };
  clip?: number | null;
  path: number;
  /**
   * Evaluated world-width stroke; not transformed by the geometry matrix.
   */
  stroke?: StrokeStyle | null;
  /**
   * None is the identity transform.
   */
  transform?: number | null;
}
export interface Gradient {
  alpha: GradientAlpha;
  geometry: GradientGeometry;
  interpolation: GradientInterpolation;
  /**
   * Nondecreasing positions in [0,1]. Repeated positions are hard stops.
   * Missing endpoint positions extend the first/last color to that endpoint.
   */
  stops: GradientStop[];
  tile: GradientTile;
}
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
export interface GradientPlane {
  origin: Point1;
  tileX: GradientAxisTile;
  tileY: GradientAxisTile;
  uncertainty?: GradientPlaneUncertainty | null;
  xStep: Point3;
  yStep: Point;
}
/**
 * World Q32 EMU position of the unit tile's top-left corner.
 */
export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface GradientPlaneUncertainty {
  origin: Point2;
  xStep: Point;
  yStep: Point;
}
/**
 * Nonnegative Q32 world EMU error per input parameter.
 */
export interface Point2 {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * World displacement for a unit change in each tile coordinate.
 */
export interface Point3 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface GradientStop {
  position: number;
  /**
   * Straight sRGB working channels. Finite RGB may exceed [0,1]; alpha must
   * be in [0,1]. No RGBA8 quantization is performed before interpolation.
   *
   * @minItems 4
   * @maxItems 4
   */
  srgb: [number, number, number, number];
}
export interface ImageBrush {
  origin: Point4;
  resource: number;
  sampling: ImageSampling;
  /**
   * Optional continuous source pixel domain; may extend beyond the resource
   * into transparent space. Coordinates are Q32 pixels, not world EMU.
   */
  sourceDomain?: ImageSourceDomain | null;
  tileX: ImageTile;
  tileY: ImageTile;
  /**
   * Outward errors from upstream layout/placement. These participate in the
   * same device budget as float conversion, including repeated tile phase.
   */
  uncertainty?: ImageBrushUncertainty | null;
  xStep: Point7;
  yStep: Point;
}
/**
 * World Q32 EMU position of source pixel boundary (0, 0).
 */
export interface Point4 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface ImageSourceDomain {
  bottom: FixedQ32;
  left: FixedQ32;
  right: FixedQ32;
  top: FixedQ32;
}
export interface ImageBrushUncertainty {
  origin: Point5;
  /**
   * Nonnegative Q32 source pixel errors, left/top/right/bottom. Must be zero
   * when the brush has no explicit source domain.
   *
   * @minItems 4
   * @maxItems 4
   */
  sourceDomain: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  xStep: Point6;
  yStep: Point;
}
/**
 * Nonnegative Q32 world EMU errors; rebase never changes these bounds.
 */
export interface Point5 {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * Nonnegative Q32 world EMU per source pixel.
 */
export interface Point6 {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * World Q32 EMU displacement per source pixel, not endpoint coordinates.
 */
export interface Point7 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface StrokeStyle {
  cap: StrokeCap;
  join: StrokeJoin;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  width: string;
}
/**
 * Render the complete interval onto transparent pixels, then source-over it
 * onto its parent with a single opacity. This is not per-paint alpha.
 */
export interface OpacityGroup {
  /**
   * Exclusive, and strictly greater than first_draw.
   */
  endDraw: number;
  firstDraw: number;
  /**
   * 0 is transparent; 65535 is opaque. Linear coverage of premultiplied sRGB.
   */
  opacity: number;
}
export interface FillPath {
  /**
   * Local Q32 EMU; open contours are implicitly closed for filling.
   */
  commands: PathCommand[];
  fillRule: FillRule;
}
export interface TransformNode {
  affine: Affine;
  /**
   * Optional outer transform. Parents must precede children.
   */
  parent?: number | null;
}
/**
 * Evaluated affine transform, not author rotation/flip/viewport semantics.
 */
export interface Affine {
  /**
   * Dimensionless Q32 in row-major order: xx, xy, yx, yy.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point8;
}
/**
 * Q32 in the same coordinate unit as the input point.
 */
export interface Point8 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface RasterViewport {
  /**
   * Straight sRGB RGBA8; output is premultiplied.
   *
   * @minItems 4
   * @maxItems 4
   */
  background: [number, number, number, number];
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateTolerance: string;
  height: number;
  origin: Point9;
  scale: PixelScale;
  width: number;
}
/**
 * Q32 EMU. Subtracted before converting to device-space float32.
 */
export interface Point9 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface PixelScale {
  denominator: number;
  /**
   * Positive rational pixels per EMU; normalized internally.
   */
  numerator: number;
}
export interface PagePlacements {
  documentSha256: Digest;
  hidden: boolean;
  pageSize: Size;
  profile: string;
  slide: SlideId;
  /**
   * Master, layout, slide contexts; not a resolved paint or placeholder list.
   */
  surfaces: PlacementSurface[];
  uniqueAngles: number;
}
export interface Size {
  height: Emu;
  width: Emu;
}
export interface PlacementSurface {
  container: ContainerId;
  /**
   * Stable author order, groups followed by their descendants. A group entry
   * is a coordinate-space record, not a paint instruction.
   */
  objects: ObjectPlacement[];
}
export interface ObjectPlacement {
  affine: Affine1;
  anchor: Point10;
  depth: number;
  object: ObjectId;
  parent: ContainerId;
  sourceSize: Size1;
  uncertainty: AffineUncertainty;
}
/**
 * Evaluated affine transform, not author rotation/flip/viewport semantics.
 */
export interface Affine1 {
  /**
   * Dimensionless Q32 in row-major order: xx, xy, yx, yy.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point8;
}
/**
 * Subtract this exact source center before applying the evaluated affine.
 */
export interface Point10 {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * Shape/picture/connector bounds are local 0..size; custom paths and groups
 * use their own viewport. The current author model has no child offset.
 */
export interface Size1 {
  height: Emu;
  width: Emu;
}
/**
 * Covers author scale/angle computation and final Q32 quantization. It is
 * additional to mo-render's bounds for an already-quantized affine graph.
 */
export interface AffineUncertainty {
  /**
   * Nonnegative outward bounds, raw dimensionless Q32.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point11;
}
/**
 * Nonnegative outward bounds, raw Q32 EMU.
 */
export interface Point11 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface PlaybackRasterInfo {
  frame: EvaluatedFrame;
  page: PageRasterInfo;
  profile: string;
}
export interface PageRasterInfo {
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  combinedCoordinateErrorBound: string;
  page: PagePlanInfo;
  scene: SceneRasterInfo;
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
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
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
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateErrorBound: string;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
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
