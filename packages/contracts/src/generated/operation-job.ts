/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ContractVersion".
 */
export type ContractVersion = "musteroffice.operations/1-draft";
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "UnixMillis".
 */
export type UnixMillis = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "JobFence".
 */
export type JobFence = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "TerminalResult".
 */
export type TerminalResult =
  | {
      outcome: "succeeded";
      receipt: OperationReceipt;
    }
  | {
      error: Failure;
      outcome: "failed";
    };
/**
 * The mutation wire shape is preserved for existing durable receipts. Both
 * alternatives deny unknown fields and have distinct required members.
 *
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "OperationReceipt".
 */
export type OperationReceipt = MutationReceipt | ExportReceipt;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "MasterId".
 */
export type MasterId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "AssetRole".
 */
export type AssetRole =
  | "editable-document"
  | "pptx"
  | "preview"
  | "quality-report"
  | "image"
  | "font"
  | "audio"
  | "video"
  | "model3d"
  | "embedded"
  | "source"
  | "other";
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ClaimBasis".
 */
export type ClaimBasis = "none" | "static-inspection" | "roundtrip" | "application-test";
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ClaimKind".
 */
export type ClaimKind = "structure" | "layout" | "native-editability" | "playback" | "target-application";
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ClaimStatus".
 */
export type ClaimStatus = "passed" | "failed" | "not_proven" | "not_applicable";
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "PreviewSample".
 */
export type PreviewSample = {
  mode: "editor";
};
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
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
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "JobState".
 */
export type JobState = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export interface JobInfo {
  cancelRequested: boolean;
  contractVersion: ContractVersion;
  createdAt: UnixMillis;
  documentId: DocumentId;
  executorDigest: Digest;
  fence: JobFence;
  id: RequestId;
  leaseUntil?: UnixMillis | null;
  operation: string;
  requestDigest: Digest;
  requestId: RequestId;
  /**
   * Receipts remain durable until explicit host lifecycle management; there is
   * no advertised expiry or automatic deletion of committed documents.
   */
  result?: TerminalResult | null;
  state: JobState;
  updatedAt: UnixMillis;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "MutationReceipt".
 */
export interface MutationReceipt {
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
  transaction?: TransactionReceipt | null;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
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
 * This interface was referenced by `JobInfo`'s JSON-Schema
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
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "AnchorMap".
 */
export interface AnchorMap {
  deleted: number;
  inserted: number;
  paragraph: ParagraphId;
  start: number;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "ExportReceipt".
 */
export interface ExportReceipt {
  bundle: DeliveryBundle;
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "DeliveryBundle".
 */
export interface DeliveryBundle {
  assets: DeliveryAsset[];
  claims: Claim[];
  document: DeliveredDocument;
  pptxAssetId: RequestId;
  previews: Preview[];
  profileId: string;
  version: string;
  versions: Versions;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "DeliveryAsset".
 */
export interface DeliveryAsset {
  byteLength: ByteLength;
  id: RequestId;
  mediaType: string;
  role: AssetRole;
  sha256: Digest;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "Claim".
 */
export interface Claim {
  basis: ClaimBasis;
  evidenceAssetIds: RequestId[];
  kind: ClaimKind;
  profileId: string;
  reason?: string | null;
  status: ClaimStatus;
  subjectSha256: Digest;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "DeliveredDocument".
 */
export interface DeliveredDocument {
  documentId: DocumentId;
  modelAssetId: RequestId;
  revision: Digest;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "Preview".
 */
export interface Preview {
  height: number;
  imageAssetId: RequestId;
  pageId: SlideId;
  sample: PreviewSample;
  width: number;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "Versions".
 */
export interface Versions {
  documentSchema: string;
  engine: string;
  featureRegistrySha256: Digest;
  fontProfileSha256: Digest;
  operationSchema: string;
  rules: string;
}
/**
 * This interface was referenced by `JobInfo`'s JSON-Schema
 * via the `definition` "Failure".
 */
export interface Failure {
  code: FailureCode;
  detail?: unknown;
  message: string;
}
