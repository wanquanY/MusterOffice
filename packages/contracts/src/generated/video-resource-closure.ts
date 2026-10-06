/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ResourceClosure`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `ResourceClosure`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `ResourceClosure`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

export interface ResourceClosure {
  documentDigest: Digest;
  resources: ResourceRequirement[];
}
/**
 * This interface was referenced by `ResourceClosure`'s JSON-Schema
 * via the `definition` "ResourceRequirement".
 */
export interface ResourceRequirement {
  byteLength: ByteLength;
  digest: Digest;
  resourceId: ResourceId;
}
