/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { BlendMode, ColorDependency, ColorNotice, ColorSample, Digest, EffectiveGradientShade, EffectiveImageMode, EffectiveLineDash, EffectiveLineFill, EffectiveLineJoin, Emu, FillOrigin, FillOutcome, FillPaintColors, FillTarget, FixedQ32, GeometryOrigin, GradientAlpha, GradientGeometry, LineOrigin, LineOutcome, LinePaintColor, NativeBlipCompression, NativeCompoundLine, NativeCoordinate, NativeFillAlignment, NativeLineCap, NativeLineEnd, NativeLineEndSize, NativePathShade, NativePattern, NativePenAlignment, NativePercentage, NativePresetDash, NativeTileFlip, PagePaintKind, SourceColorTransform, SourceColorValue, SourcePageProfile, SourcePlaceholderMatch, SurfaceKind, TransformValueSource } from './part-001.js';

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
      to: Point1;
    }
  | {
      kind: "line";
      to: Point1;
    }
  | {
      control: Point1;
      kind: "quadratic";
      to: Point1;
    }
  | {
      control1: Point1;
      control2: Point1;
      kind: "cubic";
      to: Point1;
    }
  | {
      kind: "close";
    };

export type FillRule = "nonzero" | "evenodd";

export type PptxPageFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED"
  | "COORDINATE_RANGE"
  | "PRECISION_EXCEEDED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";

export type SourcePageIssue =
  | {
      issue: SourceVisualIssue;
      kind: "visual";
    }
  | {
      kind: "object";
      nativeKind: SourceObjectKind;
    }
  | {
      kind: "text";
      sourceOrdinal: number;
    }
  | {
      kind: "effects";
      sourceOrdinal: number;
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
    }
  | {
      kind: "specialPlaceholder";
    }
  | {
      kind: "placement";
      reason: PlacementUnresolved;
    }
  | {
      kind: "geometry";
      reason: GeometryUnresolved;
    }
  | {
      kind: "fill";
    }
  | {
      kind: "line";
    }
  | {
      kind: "pathFillModifier";
    }
  | {
      kind: "fillSpace";
      redirects: FillRedirect[];
    };

export type SourceVisualIssueKind = "element" | "attribute";

export type SourceObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";

export type PlacementCause =
  | {
      kind: "missingOrigin";
    }
  | {
      kind: "missingSize";
    }
  | {
      kind: "inheritance";
      status: SourcePlaceholderMatch;
    }
  | {
      kind: "retainedTransform";
      sourceOrdinal: number;
    }
  | {
      field: string;
      kind: "invalidCoordinate";
    }
  | {
      kind: "numericRange";
    };

export type GeometryUnresolved =
  | {
      kind: "missingDeclaration";
    }
  | {
      kind: "unsupportedObject";
    }
  | {
      kind: "missingExtent";
    }
  | {
      kind: "invalidExtent";
    }
  | {
      kind: "retainedContent";
      origin: GeometryOrigin;
    }
  | {
      issue: FormulaIssue;
      kind: "formula";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidCoordinate";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidAngle";
      origin: GeometryOrigin;
    }
  | {
      kind: "unknownReference";
      origin: GeometryOrigin;
      token: string;
    }
  | {
      kind: "reservedGuide";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidGuideName";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidHandleReference";
      origin: GeometryOrigin;
    }
  | {
      kind: "numericRange";
      origin: GeometryOrigin;
    };

export type FormulaIssue = "unknownOperation" | "arity" | "divisionByZero" | "undefinedDirection" | "tangentPole";

export interface SourcePagePlan {
  bindings: SourcePagePaintBinding[];
  deviceWork: SceneWork;
  downstreamCoordinateErrorBound: FixedQ32;
  info: SourcePageInfo;
  paintSources: SourcePagePaintSource[];
  raster: SceneRasterRequest;
}

export interface SourcePagePaintBinding {
  /**
   * The object's own drawing surface supplies ordinary color/style context.
   * useBgFill explicitly redirects to the consuming slide's background.
   * Placeholder inheritance retains original declaration owners.
   */
  drawingSurface: string;
  fill: SourceFillColorResult;
  line?: SourceLineColorResult | null;
  location: SourcePageLocation;
  /**
   * p:blipFill is a second fill above spPr fill and below the outline.
   */
  pictureFill?: SourceFillColorResult | null;
  placement?: NativePlacement | null;
}

export interface SourceFillColorResult {
  colors: FillPaintColors;
  /**
   * A background redirect may change context within the same drawing batch.
   * Absent means the enclosing batch's color mapping and scheme apply.
   */
  contextOverride?: FillColorSurface | null;
  style: FillOutcome;
  target: FillTarget;
}

export interface FillColorEvaluation {
  dependencies: ColorDependency[];
  notices: ColorNotice[];
  outcome: ColorSample;
  /**
   * Present only when evaluation actually consulted a native reference.
   */
  placeholder?: FillPlaceholderBinding | null;
}

export interface FillPlaceholderBinding {
  colorOrdinal?: number | null;
  owner: FillOwner;
  referenceOrdinal: number;
}

export interface FillOwner {
  part: string;
  target: FillTarget;
}

export interface FillColorSurface {
  colorMapping?: SourceColorMapRef | null;
  colorScheme?: SourceThemeSchemeRef | null;
  surface: string;
}

export interface SourceColorMapRef {
  part: string;
  sourceOrdinal: number;
}

export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}

export interface FillColorExpression {
  color: FillColorTerm;
  /**
   * Resolve a style reference color in this native owner only if color
   * evaluation actually reaches phClr. Inheritance does not sample colors.
   */
  contextOwner?: FillOwner | null;
}

export interface FillColorTerm {
  declaredBy: FillOrigin;
  transforms: SourceColorTransform[];
  value: SourceColorValue;
}

export interface EffectiveGradientFill {
  flip: FillValue6;
  rotateWithShape: FillValue4;
  shade: EffectiveGradientShade;
  stops: FillValue;
  tileRect: EffectiveFillRect;
}

export interface FillValue6 {
  declaredBy: FillOrigin;
  value: NativeTileFlip;
}

export interface FillValue4 {
  declaredBy: FillOrigin;
  value: boolean;
}

export interface FillValue3 {
  declaredBy: FillOrigin;
  value: number;
}

export interface EffectiveFillRect {
  bottom: FillValue2;
  declaredBy: FillOrigin;
  left: FillValue2;
  right: FillValue2;
  top: FillValue2;
}

export interface FillValue2 {
  declaredBy: FillOrigin;
  value: NativePercentage;
}

export interface FillValue5 {
  declaredBy: FillOrigin;
  value: NativePathShade;
}

export interface FillValue {
  declaredBy: FillOrigin;
  value: EffectiveGradientStop[];
}

export interface EffectiveGradientStop {
  color: FillColorExpression;
  position: FillValue2;
}

export interface EffectivePatternFill {
  background: FillColorExpression;
  foreground: FillColorExpression;
  preset: FillValue7;
}

export interface FillValue7 {
  declaredBy: FillOrigin;
  value: NativePattern;
}

export interface EffectiveImageFill {
  compression: FillValue9;
  dpi: FillValue3;
  embed: FillValue8;
  link: FillValue81;
  mode: EffectiveImageMode;
  rotateWithShape: FillValue4;
  sourceRect: EffectiveFillRect;
}

export interface FillValue9 {
  declaredBy: FillOrigin;
  value: NativeBlipCompression;
}

/**
 * Each relationship belongs to the part in its own declaring origin.
 * Empty strings are explicit or profile-default empty relationship IDs.
 */
export interface FillValue8 {
  declaredBy: FillOrigin;
  value: string;
}

export interface FillValue81 {
  declaredBy: FillOrigin;
  value: string;
}

export interface EffectiveFillTile {
  alignment: FillValue11;
  flip: FillValue6;
  scaleX: FillValue2;
  scaleY: FillValue2;
  translateX: FillValue10;
  translateY: FillValue10;
}

export interface FillValue11 {
  declaredBy: FillOrigin;
  value: NativeFillAlignment;
}

export interface FillValue10 {
  declaredBy: FillOrigin;
  value: NativeCoordinate;
}

export interface FillRedirect {
  declaredBy: FillOrigin;
  target: FillOwner;
}

export interface SourceObjectRef {
  nativeId: number;
  part: string;
}

export interface SourceLineColorResult {
  nativeId: number;
  paint: LinePaintColor;
  style: LineOutcome;
}

export interface EffectiveLine {
  alignment: LineValue4;
  cap: LineValue2;
  compound: LineValue3;
  dash: EffectiveLineDash;
  fill: EffectiveLineFill;
  head: EffectiveLineEnd;
  join: EffectiveLineJoin;
  tail: EffectiveLineEnd;
  width: LineValue;
}

export interface LineValue4 {
  declaredBy: LineOrigin;
  value: NativePenAlignment;
}

export interface LineValue2 {
  declaredBy: LineOrigin;
  value: NativeLineCap;
}

export interface LineValue3 {
  declaredBy: LineOrigin;
  value: NativeCompoundLine;
}

export interface LineValue5 {
  declaredBy: LineOrigin;
  value: NativePresetDash;
}

export interface SourceDashStop {
  dash: NativePercentage;
  sourceOrdinal: number;
  space: NativePercentage;
}

/**
 * Retains native color operations and their owning style context. Colors are
 * not sampled/quantized during line inheritance; color evaluation is separate.
 */
export interface LineColorExpression {
  color: LineColorTerm;
  placeholder?: LineColorTerm | null;
}

export interface LineColorTerm {
  declaredBy: LineOrigin;
  transforms: SourceColorTransform[];
  value: SourceColorValue;
}

export interface EffectiveLineEnd {
  declaredBy: LineOrigin;
  kind: LineValue7;
  length: LineValue8;
  width: LineValue8;
}

export interface LineValue7 {
  declaredBy: LineOrigin;
  value: NativeLineEnd;
}

export interface LineValue8 {
  declaredBy: LineOrigin;
  value: NativeLineEndSize;
}

export interface LineValue6 {
  declaredBy: LineOrigin;
  value: NativePercentage;
}

export interface LineValue {
  declaredBy: LineOrigin;
  value: Emu;
}

export interface SourcePageLocation {
  /**
   * None denotes a surface/background declaration.
   */
  object?: number | null;
  part: string;
}

export interface NativePlacement {
  affine: Affine;
  anchor: Point1;
  sourceOrigin: Point2;
  sourceSize: Size;
  transform: ResolvedNativeTransform;
  uncertainty: AffineUncertainty;
}

/**
 * Already in surface coordinates; subtract anchor, then apply once.
 */
export interface Affine {
  /**
   * Dimensionless Q32 in row-major order: xx, xy, yx, yy.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point;
}

/**
 * Q32 in the same coordinate unit as the input point.
 */
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}

export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}

export interface Point2 {
  x: Emu;
  y: Emu;
}

/**
 * Effective coordinate viewport. A zero/missing group child extent uses
 * the corresponding target extent, producing unit scale on that axis.
 */
export interface Size {
  height: Emu;
  width: Emu;
}

export interface ResolvedNativeTransform {
  childOrigin?: TransformValue | null;
  childSize?: TransformValue2 | null;
  flipHorizontal: TransformValue4;
  flipVertical: TransformValue4;
  /**
   * Office ignores a graphic frame's own orientation attributes; parent
   * group orientation still applies. The original values remain above.
   */
  graphicFrameOrientationIgnored: boolean;
  origin: TransformValue;
  rotation: TransformValue3;
  size: TransformValue2;
}

export interface TransformValue {
  source: TransformValueSource;
  value: Point2;
}

export interface TransformValue2 {
  source: TransformValueSource;
  value: Size1;
}

export interface Size1 {
  height: Emu;
  width: Emu;
}

export interface TransformValue4 {
  source: TransformValueSource;
  value: boolean;
}

export interface TransformValue3 {
  source: TransformValueSource;
  value: number;
}

export interface AffineUncertainty {
  /**
   * Nonnegative outward bounds, raw dimensionless Q32.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point3;
}

/**
 * Nonnegative outward bounds, raw Q32 EMU.
 */
export interface Point3 {
  x: FixedQ32;
  y: FixedQ32;
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

export interface SourcePageInfo {
  arcSegments: number;
  generatedCommands: number;
  hiddenSlide: boolean;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  imageClipCoordinateErrorBound?: string;
  layers: SourcePageLayer[];
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  pathCoordinateErrorBound: string;
  placementCoordinateErrorBound: FixedQ32;
  profile: SourcePageProfile;
  slide: string;
  sourceSha256: Digest;
}

export interface SourcePageLayer {
  hiddenObjects: number[];
  kind: SurfaceKind;
  objects: number[];
  part: string;
  templatePlaceholders: number[];
  visible: boolean;
}

export interface SourcePagePaintSource {
  binding: number;
  /**
   * Present for the distinct p:blipFill paint. Ordinary shape/text paints
   * retain their existing provenance through binding/path/text_sources.
   */
  fillTarget?: FillTarget | null;
  instance: number;
  paint: PagePaintKind;
  path?: GeometryOrigin | null;
}

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

export interface GradientPlane {
  origin: Point4;
  tileX: GradientAxisTile;
  tileY: GradientAxisTile;
  uncertainty?: GradientPlaneUncertainty | null;
  xStep: Point6;
  yStep: Point1;
}

/**
 * World Q32 EMU position of the unit tile's top-left corner.
 */
export interface Point4 {
  x: FixedQ32;
  y: FixedQ32;
}

export interface GradientPlaneUncertainty {
  origin: Point5;
  xStep: Point1;
  yStep: Point1;
}

/**
 * Nonnegative Q32 world EMU error per input parameter.
 */
export interface Point5 {
  x: FixedQ32;
  y: FixedQ32;
}

/**
 * World displacement for a unit change in each tile coordinate.
 */
export interface Point6 {
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
  origin: Point7;
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
  xStep: Point10;
  yStep: Point1;
}

/**
 * World Q32 EMU position of source pixel boundary (0, 0).
 */
export interface Point7 {
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
  origin: Point8;
  /**
   * Nonnegative Q32 source pixel errors, left/top/right/bottom. Must be zero
   * when the brush has no explicit source domain.
   *
   * @minItems 4
   * @maxItems 4
   */
  sourceDomain: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  xStep: Point9;
  yStep: Point1;
}

/**
 * Nonnegative Q32 world EMU errors; rebase never changes these bounds.
 */
export interface Point8 {
  x: FixedQ32;
  y: FixedQ32;
}

/**
 * Nonnegative Q32 world EMU per source pixel.
 */
export interface Point9 {
  x: FixedQ32;
  y: FixedQ32;
}

/**
 * World Q32 EMU displacement per source pixel, not endpoint coordinates.
 */
export interface Point10 {
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
  affine: Affine1;
  /**
   * Optional outer transform. Parents must precede children.
   */
  parent?: number | null;
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
  translation: Point;
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
  origin: Point11;
  scale: PixelScale;
  width: number;
}

/**
 * Q32 EMU. Subtracted before converting to device-space float32.
 */
export interface Point11 {
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

export interface PptxPageFailure {
  code: PptxPageFailureCode;
  issue?: SourcePageIssue | null;
  location?: SourcePageLocation | null;
  message: string;
}

export interface SourceVisualIssue {
  kind: SourceVisualIssueKind;
  localName: string;
  namespace: string;
  sourceOrdinal: number;
}

export interface PlacementUnresolved {
  cause: PlacementCause;
  object: SourceObjectRef;
}
