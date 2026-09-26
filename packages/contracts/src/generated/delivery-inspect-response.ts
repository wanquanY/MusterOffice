/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type DeliveryInspectResponse =
  | {
      report: ReceiptInspection;
      status: "inspected";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type Digest = string;
export type ClaimBasis = "none" | "static-inspection" | "roundtrip" | "application-test";
export type RequestId = string;
export type ClaimKind = "structure" | "layout" | "native-editability" | "playback" | "target-application";
export type ClaimStatus = "passed" | "failed" | "not_proven" | "not_applicable";
export type DocumentId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

/**
 * A diagnostic report, not a transferable grant or a substitute for a commit.
 * Claims remain producer declarations; this checker does not upgrade them.
 */
export interface ReceiptInspection {
  assetsVerified: number;
  bundleDigest: Digest;
  declaredClaims: Claim[];
  documentId: DocumentId;
  pages: number;
  profile: string;
  revision: Digest;
  semanticDigest: Digest;
  settingsDigest: Digest;
  totalBytes: ByteLength;
}
export interface Claim {
  basis: ClaimBasis;
  evidenceAssetIds: RequestId[];
  kind: ClaimKind;
  profileId: string;
  reason?: string | null;
  status: ClaimStatus;
  subjectSha256: Digest;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
