/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
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
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "ClaimBasis".
 */
export type ClaimBasis = "none" | "static-inspection" | "roundtrip" | "application-test";
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "ClaimKind".
 */
export type ClaimKind = "structure" | "layout" | "native-editability" | "playback" | "target-application";
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "ClaimStatus".
 */
export type ClaimStatus = "passed" | "failed" | "not_proven" | "not_applicable";
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "PreviewSample".
 */
export type PreviewSample = {
  mode: "editor";
};

export interface DeliveryInspectRequest {
  bundle: DeliveryBundle;
  contents: DeliveryContentRange[];
  expected: DeliveryExpectation;
}
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
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
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
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
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
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
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "DeliveredDocument".
 */
export interface DeliveredDocument {
  documentId: DocumentId;
  modelAssetId: RequestId;
  revision: Digest;
}
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
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
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
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
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "DeliveryContentRange".
 */
export interface DeliveryContentRange {
  assetId: RequestId;
  byteLength: ByteLength;
  byteOffset: ByteLength;
}
/**
 * Pins come from the accepted operation, not from this delivery's own claims.
 * The product separately binds them to its invocation, generation and fence.
 *
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "DeliveryExpectation".
 */
export interface DeliveryExpectation {
  documentId: DocumentId;
  renderer: RendererIdentity;
  revision: Digest;
  semanticDigest: Digest;
  settingsDigest: Digest;
}
/**
 * This interface was referenced by `DeliveryInspectRequest`'s JSON-Schema
 * via the `definition` "RendererIdentity".
 */
export interface RendererIdentity {
  implementationSha256: Digest;
  profile: string;
}
