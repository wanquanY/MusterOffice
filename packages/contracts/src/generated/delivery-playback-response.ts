/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type DeliveryPlaybackResponse =
  | {
      inputs: DeliveryPlaybackInputs;
      status: "prepared";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type DocumentId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type RequestId = string;
export type AssetRole =
  | "editable-document"
  | "pptx"
  | "preview"
  | "quality-report"
  | "image"
  | "font"
  | "audio"
  | "video"
  | "model3d"
  | "embedded"
  | "source"
  | "other";
export type Digest = string;
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
export type SlideId = string;
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
export type ImageSampling = "nearest" | "linear";
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

/**
 * Byte identities and computation requests, not a grant, cached plan, or proof
 * that every animation in the source is supported. Playback preparation still
 * validates the selected source, explicit fonts and timing profile.
 */
export interface DeliveryPlaybackInputs {
  documentId: DocumentId;
  fontBundle?: DeliveryAsset | null;
  fonts?: FontManifest | null;
  /**
   * In presentation order; source part names come from the inspected OPC
   * relationships and are never guessed from the page ordinal.
   */
  pages: DeliveryPlaybackPage[];
  profile: string;
  /**
   * Edited document provenance. Source playback bindings use source.sha256,
   * which remains distinct from this document revision.
   */
  revision: string;
  source: DeliveryAsset;
}
export interface DeliveryAsset {
  byteLength: ByteLength;
  id: RequestId;
  mediaType: string;
  role: AssetRole;
  sha256: Digest;
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
export interface DeliveryPlaybackPage {
  pageId: SlideId;
  request: ResourcePageRequest;
}
/**
 * Per-page input; immutable document and font resources belong to the session.
 */
export interface ResourcePageRequest {
  imageSource: ImageSourceSelection;
  page: SourcePageRequest;
  sampling: ImageSampling;
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
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
