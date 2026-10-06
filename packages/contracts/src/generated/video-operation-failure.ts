/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "FailureCode".
 */
export type FailureCode =
  | "INPUT_INVALID"
  | "SNAPSHOT_CORRUPT"
  | "STALE_SNAPSHOT"
  | "CONFLICT"
  | "LOCKED"
  | "LINKED_SELECTION"
  | "NOT_FOUND"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "OUTSIDE_PROJECT"
  | "INTERNAL_INVARIANT";
/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "ObjectKey".
 */
export type ObjectKey =
  | {
      kind: "header";
    }
  | {
      id: ObjectId;
      kind: "sequence";
    }
  | {
      id: ObjectId;
      kind: "track";
    }
  | {
      id: ObjectId;
      kind: "clip";
    }
  | {
      id: ResourceId;
      kind: "resource";
    }
  | {
      id: ObjectId;
      kind: "linkGroup";
    };
/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "ValidationCode".
 */
export type ValidationCode =
  | "invalid_value"
  | "missing_reference"
  | "duplicate_identity"
  | "ownership_conflict"
  | "reference_cycle"
  | "limit_exceeded"
  | "identity_mismatch"
  | "incompatible_track"
  | "overlap";

export interface Failure {
  code: FailureCode;
  message: string;
  objects: ObjectKey[];
  operation?: number | null;
  validation?: ValidationReport | null;
}
/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "ValidationReport".
 */
export interface ValidationReport {
  issues: ValidationIssue[];
  truncated: boolean;
}
/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "ValidationIssue".
 */
export interface ValidationIssue {
  code: ValidationCode;
  message: string;
  path: string;
}
