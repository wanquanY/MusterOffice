/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ImageDecodeResponse =
  | {
      info: DecodedImageInfo;
      status: "decoded";
    }
  | {
      error: ImageFailure;
      status: "error";
    };
export type ImageFormat = "png" | "jpeg";
export type Digest = string;
export type ResolutionSource = "pngPhysical" | "jfif" | "exifIfd0";
export type ResolutionUnit = "aspectRatio" | "inch" | "centimetre" | "metre";
export type PhysicalPixelSize =
  | {
      status: "unspecified";
    }
  | {
      status: "zeroDensity";
    }
  | {
      status: "conflicting";
    }
  | {
      status: "known";
      x: PixelExtent;
      y: PixelExtent;
    };
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type SourceColor = "assumedSrgb" | "icc" | "pngColor";
export type ImageFailureCode =
  | "INPUT_INVALID"
  | "LIMIT_EXCEEDED"
  | "UNSUPPORTED"
  | "CANCELLED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";

export interface DecodedImageInfo {
  byteLength: number;
  encodedBitDepth: number;
  encodedHeight: number;
  encodedWidth: number;
  format: ImageFormat;
  height: number;
  orientation: number;
  pixelsSha256: Digest;
  profile: string;
  resolution: ImageResolution;
  sourceColor: SourceColor;
  sourceSha256: Digest;
  width: number;
}
export interface ImageResolution {
  /**
   * Raw fields in encoded axes. Missing EXIF fields remain null; derivation
   * uses Exif's specified defaults (72, 72, inch), never a host DPI default.
   */
  declarations: ResolutionDeclaration[];
  physicalPixelSize: PhysicalPixelSize;
}
export interface ResolutionDeclaration {
  source: ResolutionSource;
  /**
   * Absolute byte offset of the PNG chunk or JPEG marker in source bytes.
   */
  sourceOffset: number;
  /**
   * None preserves an absent EXIF ResolutionUnit. Derivation applies the
   * Exif specification's inch default without rewriting this declaration.
   */
  unit?: ResolutionUnit | null;
  x?: Density | null;
  y?: Density | null;
}
export interface Density {
  denominator: number;
  numerator: number;
}
/**
 * Reduced, positive rational EMU per normalized output pixel.
 */
export interface PixelExtent {
  denominator: number;
  numerator: Emu;
}
export interface ImageFailure {
  code: ImageFailureCode;
  message: string;
}
