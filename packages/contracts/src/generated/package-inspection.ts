/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PackageInspectionResponse =
  | {
      report: PackageReport;
      status: "inspected";
    }
  | {
      error: PackageError;
      status: "error";
    };
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type PackageReportFormat = "musteroffice.opc-inspection/1";
export type Digest = string;
export type PackageTarget =
  | {
      fragment?: string | null;
      kind: "internal";
      part: string;
    }
  | {
      kind: "external";
    };
export type PackageSource =
  | {
      kind: "package";
    }
  | {
      kind: "part";
      name: string;
    };
export type PackageErrorCode =
  "INPUT_INVALID" | "LIMIT_EXCEEDED" | "CANCELLED" | "UNSUPPORTED" | "PRESERVATION_CONFLICT" | "READ_FAILED";

export interface PackageReport {
  byteLength: ByteLength;
  /**
   * True means signature-related parts/relationships exist, not signature validity.
   */
  containsSignatures: boolean;
  format: PackageReportFormat;
  parts: PackagePart[];
  relationships: RelationshipGroup[];
  sha256: Digest;
}
export interface PackagePart {
  byteLength: ByteLength;
  compressedLength: ByteLength;
  contentType: string;
  name: string;
  sha256: Digest;
}
export interface RelationshipGroup {
  relationships: PackageRelationship[];
  source: PackageSource;
}
export interface PackageRelationship {
  id: string;
  relationshipType: string;
  resolved: PackageTarget;
  target: string;
}
export interface PackageError {
  code: PackageErrorCode;
  message: string;
}
