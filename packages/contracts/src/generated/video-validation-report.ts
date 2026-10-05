/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ValidationReport`'s JSON-Schema
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

export interface ValidationReport {
  issues: ValidationIssue[];
  truncated: boolean;
}
/**
 * This interface was referenced by `ValidationReport`'s JSON-Schema
 * via the `definition` "ValidationIssue".
 */
export interface ValidationIssue {
  code: ValidationCode;
  message: string;
  path: string;
}
