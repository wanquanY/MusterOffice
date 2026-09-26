/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SchemaDocument`'s JSON-Schema
 * via the `definition` "SchemaId".
 */
export type SchemaId =
  | "host-request"
  | "host-response"
  | "operation-request"
  | "operation-job"
  | "host-capabilities"
  | "schema-document"
  | "document"
  | "upload-request"
  | "upload-info"
  | "asset-info";
/**
 * This interface was referenced by `SchemaDocument`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface SchemaDocument {
  /**
   * Canonical domain-bound ID+schema digest, not a digest of formatted JSON.
   */
  digest: string;
  id: SchemaId;
  schema: unknown;
}
