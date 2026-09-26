/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `ValidationReport`'s JSON-Schema
 * via the `definition` "ValidationCode".
 */
export type ValidationCode =
  | "IDENTITY_MISMATCH"
  | "MISSING_REFERENCE"
  | "DUPLICATE_IDENTITY"
  | "INVALID_VALUE"
  | "OWNERSHIP_CONFLICT"
  | "REFERENCE_CYCLE"
  | "LIMIT_EXCEEDED";

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
