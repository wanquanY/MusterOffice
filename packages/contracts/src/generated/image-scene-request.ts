/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageAlpha".
 */
export type ImageAlpha = "straight" | "premultiplied";
/**
 * Porter-Duff composition, separate from the source brush and geometry mask.
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "BlendMode".
 */
export type BlendMode = "sourceOver" | "source";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientAlpha".
 */
export type GradientAlpha = "straight" | "premultiplied";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientGeometry".
 */
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
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientField".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientAxisTile".
 */
export type GradientAxisTile = "clamp" | "repeat" | "mirror";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientInterpolation".
 */
export type GradientInterpolation = ("srgb" | "linearSrgb") | "officeGamma1875";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientTile".
 */
export type GradientTile = "clamp" | "repeat" | "mirror" | "decal";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageSampling".
 */
export type ImageSampling = "nearest" | "linear";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageTile".
 */
export type ImageTile = "clamp" | "repeat" | "mirror" | "decal";
/**
 * The source canvas at a capture point. Explicit captures may be consumed
 * later in any group; their pixels remain immutable after the source closes.
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "SnapshotScope".
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "StrokeCap".
 */
export type StrokeCap = "butt" | "round" | "square";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "StrokeJoin".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "PathCommand".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "FillRule".
 */
export type FillRule = "nonzero" | "evenodd";
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "Brush".
 */
export type Brush =
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface ImageSceneRasterRequest {
  images: ImageResource[];
  raster: SceneRasterRequest;
}
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageResource".
 */
export interface ImageResource {
  alpha: ImageAlpha;
  height: number;
  /**
   * Hash of tightly packed, row-major RGBA8 bytes for this resource.
   */
  sha256: string;
  width: number;
}
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "SceneRasterRequest".
 */
export interface SceneRasterRequest {
  scene: DrawScene;
  viewport: RasterViewport;
}
/**
 * Evaluated resources/instances, independent of authoring document semantics.
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "DrawScene".
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
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ClipNode".
 */
export interface ClipNode {
  parent?: number | null;
  path: number;
  transform?: number | null;
}
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "PathInstance".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "Gradient".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientPlane".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientPlaneUncertainty".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "GradientStop".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageBrush".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageSourceDomain".
 */
export interface ImageSourceDomain {
  bottom: FixedQ32;
  left: FixedQ32;
  right: FixedQ32;
  top: FixedQ32;
}
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "ImageBrushUncertainty".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "StrokeStyle".
 */
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
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "OpacityGroup".
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "FillPath".
 */
export interface FillPath {
  /**
   * Local Q32 EMU; open contours are implicitly closed for filling.
   */
  commands: PathCommand[];
  fillRule: FillRule;
}
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "TransformNode".
 */
export interface TransformNode {
  affine: Affine;
  /**
   * Optional outer transform. Parents must precede children.
   */
  parent?: number | null;
}
/**
 * Evaluated affine transform, not author rotation/flip/viewport semantics.
 *
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "Affine".
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "RasterViewport".
 */
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
/**
 * This interface was referenced by `ImageSceneRasterRequest`'s JSON-Schema
 * via the `definition` "PixelScale".
 */
export interface PixelScale {
  denominator: number;
  /**
   * Positive rational pixels per EMU; normalized internally.
   */
  numerator: number;
}
