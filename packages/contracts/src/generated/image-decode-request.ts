/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ImageDecodeRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface ImageDecodeRequest {
  /**
   * Minimum oriented sample grid. Omission retains exact full-resolution decoding.
   */
  minimumSize?: DecodeSize | null;
  sourceSha256: Digest;
}
/**
 * Minimum useful oriented sample grid. A decoder may return a larger native
 * grid (for example a JPEG DCT scale), but never upscale a source or undersample.
 *
 * This interface was referenced by `ImageDecodeRequest`'s JSON-Schema
 * via the `definition` "DecodeSize".
 */
export interface DecodeSize {
  height: number;
  width: number;
}
