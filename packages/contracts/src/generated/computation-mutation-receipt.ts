/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "MasterId".
 */
export type MasterId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "TemplateParameterId".
 */
export type TemplateParameterId = string;
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "LocalIdPolicy".
 */
export type LocalIdPolicy = "preserve";

export interface MutationReceipt {
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
  /**
   * Source and parameter evidence for a new independent document. It is not
   * a transaction committed against the source, nor the new document's base.
   */
  template?: InstantiationReceipt | null;
  transaction?: TransactionReceipt | null;
}
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "InstantiationReceipt".
 */
export interface InstantiationReceipt {
  /**
   * Pure parameter computation against the immutable source. This is not a
   * commit to the template or the instance. Scope mapping follows binding.
   */
  bindingTransaction?: TransactionReceipt | null;
  boundParameters: TemplateParameterId[];
  requestDigest: Digest;
  requestId: RequestId;
  revision: Digest;
  scopeMap: DocumentScopeMap;
  semanticDigest: Digest;
  source: TemplateSource;
  templateDigest: Digest;
}
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "TransactionReceipt".
 */
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
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
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
   * Global page geometry changed. Otherwise use invalidated_slides; metadata
   * and timeline edits have separate change fields and do not dirty layout.
   */
  invalidateAllLayout: boolean;
  /**
   * Surviving/new pages whose static layout dependency closure changed.
   */
  invalidatedSlides?: SlideId[];
  metadataChanged: boolean;
  slideOrderChanged: boolean;
  updatedObjects: ObjectId[];
  updatedSlides: SlideId[];
}
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "AnchorMap".
 */
export interface AnchorMap {
  deleted: number;
  inserted: number;
  paragraph: ParagraphId;
  start: number;
}
/**
 * Total mapping for whole-document instantiation: (source document, local ID)
 * becomes (instance document, same local ID). This preserves every internal
 * reference, including opaque native relationships, without an O(n) identity
 * table. It is not a mapping for copying a page into an existing document.
 *
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "DocumentScopeMap".
 */
export interface DocumentScopeMap {
  instanceDocument: DocumentId;
  localIdPolicy: LocalIdPolicy;
  sourceDocument: DocumentId;
}
/**
 * This interface was referenced by `MutationReceipt`'s JSON-Schema
 * via the `definition` "TemplateSource".
 */
export interface TemplateSource {
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
}
