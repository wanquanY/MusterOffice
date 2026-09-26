/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "AssetVerification".
 */
export type AssetVerification = "bytesSha256";
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "UnixMillis".
 */
export type UnixMillis = string;
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "FailureCode".
 */
export type FailureCode =
  | (
      | "INPUT_INVALID"
      | "NOT_AUTHORIZED"
      | "NOT_FOUND"
      | "REQUEST_ID_REUSED"
      | "REVISION_CONFLICT"
      | "REFERENCE_CONFLICT"
      | "DOCUMENT_EXISTS"
      | "LIMIT_EXCEEDED"
      | "CANCELLED"
      | "EXECUTION_INTERRUPTED"
      | "EXECUTOR_MISMATCH"
      | "STALE_EXECUTION"
      | "STORAGE_FAILURE"
      | "RESOURCE_CONFLICT"
      | "RESOURCE_EXPIRED"
      | "RESOURCE_INCOMPLETE"
      | "RESOURCE_BUSY"
      | "MAPPING_NOT_IMPLEMENTED"
      | "RENDER_FAILURE"
    )
  | "STORAGE_BUSY";
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "UploadState".
 */
export type UploadState = "uploading" | "verifying" | "sealed" | "failed" | "cancelled";

export interface UploadInfo {
  asset?: AssetInfo | null;
  chunkBytes: number;
  createdAt: UnixMillis;
  descriptor: AssetDescriptor;
  error?: Failure | null;
  expiresAt?: UnixMillis | null;
  id: RequestId;
  receivedBytes: ByteLength;
  requestId: RequestId;
  state: UploadState;
  updatedAt: UnixMillis;
}
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "AssetInfo".
 */
export interface AssetInfo {
  descriptor: AssetDescriptor;
  id: RequestId;
  verification: AssetVerification;
}
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
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
/**
 * This interface was referenced by `UploadInfo`'s JSON-Schema
 * via the `definition` "Failure".
 */
export interface Failure {
  code: FailureCode;
  detail?: unknown;
  message: string;
}
