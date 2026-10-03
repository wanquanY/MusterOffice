/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxPlaybackSessionRequest =
  | {
      operation: "prepare";
      request: PptxPlaybackPrepareRequest;
    }
  | {
      binding: PlaybackBinding;
      operation: "inspect";
    }
  | {
      binding: PlaybackBinding;
      operation: "inspectTiming";
    }
  | {
      operation: "render";
      sample: PlaybackSampleRequest;
    }
  | {
      binding: PlaybackBinding;
      expectedViewportRevision: number;
      operation: "resize";
      viewport: RasterViewport;
    }
  | {
      binding: PlaybackBinding;
      expectedViewportRevision: number;
      height: number;
      operation: "resizeToFit";
      width: number;
    }
  | {
      binding: PlaybackBinding;
      generation: PlaybackGeneration;
      operation: "advance";
    }
  | {
      binding: PlaybackBinding;
      operation: "dispose";
    };
/**
 * Canonical uint64 playback generation; never wraps.
 */
export type PlaybackGeneration = string;
export type Digest = string;
export type PlaybackSessionId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type FontManifestProfile = "explicit-font-resource-manifest-draft-v1";
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
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";
export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type PptxResourcePageProfile = "drawingml-resource-page-q32-v1-draft";
export type ImageSampling = "nearest" | "linear";
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
export type InputEvent =
  | {
      direction: NavigationDirection;
      kind: "presentationStep";
    }
  | {
      kind: "click";
      target?: ObjectId | null;
    }
  | {
      direction: NavigationDirection;
      kind: "navigation";
      target?: ObjectId | null;
    };
export type NavigationDirection = "next" | "previous";
export type ObjectId = string;

export interface PptxPlaybackPrepareRequest {
  binding: PlaybackBinding;
  page: PptxResourcePageRequest;
}
export interface PlaybackBinding {
  generation: PlaybackGeneration;
  revision: Digest;
  session: PlaybackSessionId;
}
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
export interface FontManifest {
  faces: ManifestFace[];
  fonts: CascadeFont[];
  profile: FontManifestProfile;
  typefaces: ManifestTypeface[];
}
export interface ManifestFace {
  family: FontNameBinding;
  font: number;
  postscript?: FontNameBinding | null;
  subfamily: FontNameBinding;
}
export interface FontNameBinding {
  expected: string;
  /**
   * Exact index in VerifiedFont metadata.names, preserving original record order.
   */
  record: number;
}
/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
export interface ManifestTypeface {
  bold?: ManifestInstance | null;
  boldItalic?: ManifestInstance | null;
  /**
   * Ordered, explicit coverage fallbacks into this manifest's typefaces.
   * Only these entries are tried; their own fallbacks are not expanded.
   * Each candidate uses the requested style slot without synthesis.
   */
  fallbacks?: string[];
  italic?: ManifestInstance | null;
  policy: TypefaceMappingPolicy;
  /**
   * Explicit instance selection; no synthesized bold/slant or slot fallback.
   */
  regular?: ManifestInstance | null;
  typeface: string;
}
export interface ManifestInstance {
  face: number;
  /**
   * All axes are validated, including instances unused by this paragraph.
   */
  variations: ShapeVariation[];
}
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
export interface SourcePageRequest {
  colorContext: ColorContext;
  expectedSourceSha256: Digest;
  profile: SourcePageProfile;
  slide: string;
  viewport: RasterViewport;
}
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
export interface PixelScale {
  denominator: number;
  /**
   * Positive rational pixels per EMU; normalized internally.
   */
  numerator: number;
}
export interface PlaybackSampleRequest {
  at: RationalTime;
  binding: PlaybackBinding;
  history?: EventHistory | null;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Complete prefix, explicitly including periods when no event occurred. A host
 * that cannot supply this context must not invent clicks when seeking.
 */
export interface EventHistory {
  binding: PlaybackBinding;
  events: PlaybackEvent[];
  through: RationalTime;
}
export interface PlaybackEvent {
  at: RationalTime;
  event: InputEvent;
  generation: PlaybackGeneration;
  sequence: number;
}
