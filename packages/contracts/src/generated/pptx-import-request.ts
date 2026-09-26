/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `PptxImportRequest`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `PptxImportRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `PptxImportRequest`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

export interface PptxImportRequest {
  documentId: DocumentId;
  expectedSourceSha256: Digest;
  resourceId: ResourceId;
}
