/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "FontManifestProfile".
 */
export type FontManifestProfile = "explicit-font-resource-manifest-draft-v1";
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "TypefaceMappingPolicy".
 */
export type TypefaceMappingPolicy =
  | {
      kind: "exactFamily";
    }
  | {
      kind: "substitution";
      profileSha256: Digest;
      reason: string;
    };
/**
 * Explicit source policy. An embedded snapshot is never silently substituted
 * for a requested linked source, nor does inspection grant network authority.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ImageSourceSelection".
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "SourcePageProfile".
 */
export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "FixedQ32".
 */
export type FixedQ32 = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PptxResourcePageProfile".
 */
export type PptxResourcePageProfile = "drawingml-resource-page-q32-v1-draft";
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ImageSampling".
 */
export type ImageSampling = "nearest" | "linear";
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * Canonical uint64 playback generation; never wraps.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PlaybackGeneration".
 */
export type PlaybackGeneration = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PlaybackSessionId".
 */
export type PlaybackSessionId = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "InputEvent".
 */
export type InputEvent =
  | {
      kind: "click";
      target?: ObjectId | null;
    }
  | {
      direction: NavigationDirection;
      kind: "navigation";
      target?: ObjectId | null;
    };
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "NavigationDirection".
 */
export type NavigationDirection = "next" | "previous";

export interface PptxPlaybackPageRequest {
  page: PptxResourcePageRequest;
  sample: PlaybackSampleRequest;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PptxResourcePageRequest".
 */
export interface PptxResourcePageRequest {
  /**
   * None disables source text; a visible text body then requires resources.
   * Fonts are explicit names/content ranges, never OS discovery or paths.
   */
  fonts?: FontManifest | null;
  imageSource: ImageSourceSelection;
  page: SourcePageRequest;
  profile: PptxResourcePageProfile;
  sampling: ImageSampling;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "FontManifest".
 */
export interface FontManifest {
  faces: ManifestFace[];
  fonts: CascadeFont[];
  profile: FontManifestProfile;
  typefaces: ManifestTypeface[];
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ManifestFace".
 */
export interface ManifestFace {
  family: FontNameBinding;
  font: number;
  postscript?: FontNameBinding | null;
  subfamily: FontNameBinding;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "FontNameBinding".
 */
export interface FontNameBinding {
  expected: string;
  /**
   * Exact index in VerifiedFont metadata.names, preserving original record order.
   */
  record: number;
}
/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "CascadeFont".
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ManifestTypeface".
 */
export interface ManifestTypeface {
  bold?: ManifestInstance | null;
  boldItalic?: ManifestInstance | null;
  italic?: ManifestInstance | null;
  policy: TypefaceMappingPolicy;
  /**
   * Explicit instance selection; no synthesized bold/slant or slot fallback.
   */
  regular?: ManifestInstance | null;
  typeface: string;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ManifestInstance".
 */
export interface ManifestInstance {
  face: number;
  /**
   * All axes are validated, including instances unused by this paragraph.
   */
  variations: ShapeVariation[];
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "SourcePageRequest".
 */
export interface SourcePageRequest {
  colorContext: ColorContext;
  expectedSourceSha256: Digest;
  profile: SourcePageProfile;
  slide: string;
  viewport: RasterViewport;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "ColorContext".
 */
export interface ColorContext {
  /**
   * Explicit unassociated sRGB and alpha, supplied by the owning style.
   *
   * @minItems 4
   * @maxItems 4
   */
  placeholder?: [number, number, number, number] | null;
  systemColors: {
    /**
     * @minItems 3
     * @maxItems 3
     */
    "3dDkShadow"?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    "3dLight"?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    activeBorder?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    activeCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    appWorkspace?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    background?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnFace?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnHighlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnShadow?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    captionText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    gradientActiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    gradientInactiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    grayText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    highlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    highlightText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    hotLight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveBorder?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveCaptionText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    infoBk?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    infoText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menu?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuBar?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuHighlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    scrollBar?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    window?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    windowFrame?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    windowText?: [number, number, number];
  };
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
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
   * Raw Q32 pixels; 256..=2^24 (at most 1/256 pixel).
   */
  coordinateTolerance: string;
  height: number;
  origin: Point;
  scale: PixelScale;
  width: number;
}
/**
 * Q32 EMU. Subtracted before converting to device-space float32.
 */
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PixelScale".
 */
export interface PixelScale {
  denominator: number;
  /**
   * Positive rational pixels per EMU; normalized internally.
   */
  numerator: number;
}
/**
 * For native source sampling, revision equals the source package SHA-256.
 * Product revision provenance and authorization remain the host's job.
 */
export interface PlaybackSampleRequest {
  at: RationalTime;
  binding: PlaybackBinding;
  history?: EventHistory | null;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PlaybackBinding".
 */
export interface PlaybackBinding {
  generation: PlaybackGeneration;
  revision: Digest;
  session: PlaybackSessionId;
}
/**
 * Complete prefix, explicitly including periods when no event occurred. A host
 * that cannot supply this context must not invent clicks when seeking.
 *
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "EventHistory".
 */
export interface EventHistory {
  binding: PlaybackBinding;
  events: PlaybackEvent[];
  through: RationalTime;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PlaybackEvent".
 */
export interface PlaybackEvent {
  at: RationalTime;
  event: InputEvent;
  generation: PlaybackGeneration;
  sequence: number;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "PlaybackSampleRequest".
 */
export interface PlaybackSampleRequest1 {
  at: RationalTime;
  binding: PlaybackBinding;
  history?: EventHistory | null;
}
/**
 * This interface was referenced by `PptxPlaybackPageRequest`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}
