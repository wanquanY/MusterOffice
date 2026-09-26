/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `UploadRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `UploadRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `UploadRequest`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;

export interface UploadRequest {
  descriptor: AssetDescriptor;
  requestId: RequestId;
}
/**
 * This interface was referenced by `UploadRequest`'s JSON-Schema
 * via the `definition` "AssetDescriptor".
 */
export interface AssetDescriptor {
  byteLength: ByteLength;
  /**
   * A declaration, not evidence of successful decoding or safe execution.
   */
  mediaType: string;
  sha256: Digest;
}
