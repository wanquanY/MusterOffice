/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "ContractVersion".
 */
export type ContractVersion = "musteroffice.operations/1-draft";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "OperationChannel".
 */
export type OperationChannel = "controlJson" | "binaryUpload" | "binaryRead";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "ServiceOperation".
 */
export type ServiceOperation =
  | "capabilities"
  | "schemas.get"
  | "presentations.create"
  | "presentations.apply"
  | "presentations.export"
  | "presentations.read"
  | "assets.beginUpload"
  | "assets.getUpload"
  | "assets.appendUpload"
  | "assets.sealUpload"
  | "assets.cancelUpload"
  | "assets.read"
  | "assets.readRange"
  | "jobs.get"
  | "jobs.cancel";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "OutputMode".
 */
export type OutputMode = "auto" | "sync" | "job";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "OperationProfile".
 */
export type OperationProfile = "presentations-pptx-resource-delivery-v1-draft" | "presentations-author-model-v01-draft";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "Permission".
 */
export type Permission =
  "create" | "edit" | "export" | "readDocument" | "readJob" | "cancelJob" | "writeAssets" | "readAssets";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "RevisionPolicy".
 */
export type RevisionPolicy = "newDocument" | "compareCurrentHead" | "immutableHistorical" | "readSelectedOrHead";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "UnavailableReason".
 */
export type UnavailableReason = "previewRendererNotConfigured" | "executionUnavailable";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "JobExecution".
 */
export type JobExecution = "explicitRun" | "hostScheduled";
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
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

export interface HostCapabilities {
  completeFeatureCatalogue: boolean;
  contractVersion: ContractVersion;
  executorDigest: Digest;
  fullPresentationAcceptance: boolean;
  limits: ServiceLimits;
  operations: OperationDescriptor[];
  queuedExecution: JobExecution;
  renderer?: RendererIdentity | null;
  schemas: SchemaId[];
}
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "ServiceLimits".
 */
export interface ServiceLimits {
  assetBytes: ByteLength;
  assetChunkBytes: ByteLength;
  documentsPerScope: number;
  export: ExportLimits;
  jobsPerPrincipal: number;
  leaseMs: number;
  outputsPerJob: number;
  outputsPerScope: number;
  requestBytes: ByteLength;
  scopeReservedBytes: ByteLength;
  uploadTtlMs: number;
  uploadsPerScope: number;
}
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "ExportLimits".
 */
export interface ExportLimits {
  artifacts: number;
  assetBytes: ByteLength;
  fontBytes: ByteLength;
  modelBytes: ByteLength;
  pages: number;
  totalBytes: ByteLength;
}
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "OperationDescriptor".
 */
export interface OperationDescriptor {
  available: boolean;
  channel: OperationChannel;
  operation: ServiceOperation;
  /**
   * Empty for immediate queries/resource operations. Job preference applies
   * only to document operations accepted into the durable owner.
   */
  outputModes: OutputMode[];
  profileId?: OperationProfile | null;
  requiredPermissions: Permission[];
  revisionPolicy?: RevisionPolicy | null;
  unavailableReason?: UnavailableReason | null;
}
/**
 * This interface was referenced by `HostCapabilities`'s JSON-Schema
 * via the `definition` "RendererIdentity".
 */
export interface RendererIdentity {
  implementationSha256: Digest;
  profile: string;
}
