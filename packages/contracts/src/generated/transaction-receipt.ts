/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "MasterId".
 */
export type MasterId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;

export interface TransactionReceipt {
  baseRevision: Digest;
  changes: ChangeSet;
  documentId: DocumentId;
  requestDigest: Digest;
  requestId: RequestId;
  revision: Digest;
  semanticDigest: Digest;
}
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "ChangeSet".
 */
export interface ChangeSet {
  anchorMaps: AnchorMap[];
  changedFonts: FontId[];
  changedLayouts: LayoutId[];
  changedMasters: MasterId[];
  changedResources: ResourceId[];
  changedThemes: ThemeId[];
  changedTimelines?: SlideId[];
  createdObjects: ObjectId[];
  createdSlides: SlideId[];
  deletedObjects: ObjectId[];
  deletedSlides: SlideId[];
  /**
   * Conservative until the compiler dependency index can produce a narrower closure.
   */
  invalidateAllLayout: boolean;
  metadataChanged: boolean;
  slideOrderChanged: boolean;
  updatedObjects: ObjectId[];
  updatedSlides: SlideId[];
}
/**
 * This interface was referenced by `TransactionReceipt`'s JSON-Schema
 * via the `definition` "AnchorMap".
 */
export interface AnchorMap {
  deleted: number;
  inserted: number;
  paragraph: ParagraphId;
  start: number;
}
