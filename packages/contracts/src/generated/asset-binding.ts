/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `AssetBinding`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `AssetBinding`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

export interface AssetBinding {
  assetId: RequestId;
  resourceId: ResourceId;
}
