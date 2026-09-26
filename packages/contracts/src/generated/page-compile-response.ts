/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PageCompileResponse =
  | {
      plan: PagePlan;
      status: "compiled";
    }
  | {
      error: PageFailure;
      status: "error";
    };
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type Digest = string;
export type SlideId = string;
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
