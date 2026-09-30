/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxChartPlotResponse =
  | {
      plot: SourceChartPlot;
      status: "compiled";
    }
  | {
      error: PptxChartGeometryFailure;
      status: "error";
    };
export type Digest = string;
export type ChartDataAuthority = "sourceCacheSnapshot";
export type ChartWorkbookTarget =
  | {
      byteLength: ByteLength;
      contentType: string;
      kind: "embedded";
      part: string;
      sha256: Digest;
    }
  | {
      kind: "external";
      uri: string;
    };
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type ChartPlotPaint =
  | {
      declaration: number;
      kind: "none";
    }
  | {
      color: number;
      declaration: number;
      kind: "solid";
      /**
       * @minItems 4
       * @maxItems 4
       */
      rgba8: [number, number, number, number];
    };
export type LineOrigin =
  | {
      kind: "chart";
      part: string;
      sourceOrdinal: number;
    }
  | {
      cell: SourceCellAddress;
      edge: TableCellEdge;
      kind: "tableCell";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      edge: TableStyleEdge;
      kind: "tableStyle";
      object: SourceObjectRef;
      part: string;
      region: TableStyleRegion;
      sourceOrdinal: number;
    }
  | {
      edge: TableStyleEdge;
      kind: "tableTheme";
      part: string;
      referenceOrdinal: number;
      region: TableStyleRegion;
      sourceOrdinal: number;
      styleIndex: number;
      via: SourceObjectRef;
    }
  | {
      kind: "object";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      kind: "theme";
      part: string;
      referenceOrdinal: number;
      sourceOrdinal: number;
      styleIndex: number;
      via: SourceObjectRef;
    }
  | {
      kind: "profileDefault";
    };
export type TableCellEdge = "left" | "right" | "top" | "bottom" | "topLeftToBottomRight" | "bottomLeftToTopRight";
export type TableStyleEdge =
  | "left"
  | "right"
  | "top"
  | "bottom"
  | "insideHorizontal"
  | "insideVertical"
  | "topLeftToBottomRight"
  | "topRightToBottomLeft";
export type TableStyleRegion =
  | "wholeTbl"
  | "band1H"
  | "band2H"
  | "band1V"
  | "band2V"
  | "lastCol"
  | "firstCol"
  | "lastRow"
  | "seCell"
  | "swCell"
  | "firstRow"
  | "neCell"
  | "nwCell";
export type NativePenAlignment = "ctr" | "in";
export type NativeLineCap = "flat" | "rnd" | "sq";
export type NativeCompoundLine = "sng" | "dbl" | "thickThin" | "thinThick" | "tri";
export type EffectiveLineDash =
  | {
      declaredBy: LineOrigin;
      kind: "preset";
      value: LineValue5;
    }
  | {
      declaredBy: LineOrigin;
      kind: "custom";
      stops: SourceDashStop[];
    };
export type NativePresetDash =
  | "solid"
  | "dot"
  | "dash"
  | "lgDash"
  | "dashDot"
  | "lgDashDot"
  | "lgDashDotDot"
  | "sysDash"
  | "sysDot"
  | "sysDashDot"
  | "sysDashDotDot";
/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;
export type NativeLineEnd = "none" | "triangle" | "stealth" | "diamond" | "oval" | "arrow";
export type NativeLineEndSize = "sm" | "med" | "lg";
export type EffectiveLineJoin =
  | {
      declaredBy: LineOrigin;
      kind: "round";
    }
  | {
      declaredBy: LineOrigin;
      kind: "bevel";
    }
  | {
      declaredBy: LineOrigin;
      kind: "miter";
      limit: LineValue6;
    };
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type ChartPlotProfile = "source-circular-declared-solid-plot-v1-draft";
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
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
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
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceChartPlot {
  chartPart: string;
  chartSha256: Digest;
  colorMapping?: SourceColorMapRef | null;
  colorScheme?: SourceThemeSchemeRef | null;
  /**
   * Maximum source geometry error, Q32 EMU. Raster consumers must compose
   * this with their own transform/device error before accepting a frame.
   */
  coordinateErrorBound: string;
  dataAuthority: ChartDataAuthority;
  externalData?: SourceChartExternalData | null;
  nativeKind: string;
  object: SourceObjectRef;
  plotSourceOrdinal: number;
  points: ChartPlotPoint[];
  profile: ChartPlotProfile;
  scene: DrawScene;
  series: ChartPlotSeries[];
  sourceSha256: Digest;
  themeOverride?: ChartThemeOverride | null;
}
export interface SourceColorMapRef {
  part: string;
  sourceOrdinal: number;
}
export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}
export interface SourceChartExternalData {
  /**
   * None means omitted; no eager refresh is performed by inspection.
   */
  autoUpdate?: boolean | null;
  relationshipId: string;
  sourceOrdinal: number;
  target: ChartWorkbookTarget;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface ChartPlotPoint {
  cacheSourceOrdinal: number;
  effectsSourceOrdinal: number;
  fill: ChartPlotPaint;
  line: ChartPlotPaint;
  lineGeometry: EffectiveLineGeometry;
  /**
   * None for zero-valued points; stable point identity is still retained.
   */
  path?: number | null;
  pointIndex: number;
  seriesIndex: number;
}
/**
 * Stroke geometry is independent of paint. Table lines can therefore use the
 * complete fill engine, including gradient/pattern paint, without losing dash,
 * compound, alignment, cap, join or endpoint declarations.
 */
export interface EffectiveLineGeometry {
  alignment: LineValue4;
  cap: LineValue2;
  compound: LineValue3;
  dash: EffectiveLineDash;
  head: EffectiveLineEnd;
  join: EffectiveLineJoin;
  tail: EffectiveLineEnd;
  width: LineValue;
}
export interface LineValue4 {
  declaredBy: LineOrigin;
  value: NativePenAlignment;
}
export interface SourceCellAddress {
  column: number;
  row: number;
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
export interface ChartPlotSeries {
  coordinateErrorBound: FixedQ32;
  formula?: string | null;
  index: number;
  innerRadius: FixedQ32;
  order: number;
  outerRadius: FixedQ32;
  sourceOrdinal: number;
  valuesSourceOrdinal: number;
}
export interface ChartThemeOverride {
  part: string;
  sha256: Digest;
}
export interface PptxChartGeometryFailure {
  code: PptxFailureCode;
  message: string;
  pointIndex?: number | null;
  seriesIndex?: number | null;
  sourceOrdinal?: number | null;
}
