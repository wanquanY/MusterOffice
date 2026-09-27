/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `Failure`'s JSON-Schema
 * via the `definition` "FailureCode".
 */
export type FailureCode =
  | "INPUT_INVALID"
  | "NOT_FOUND"
  | "REQUEST_ID_REUSED"
  | "REVISION_CONFLICT"
  | "REFERENCE_CONFLICT"
  | "DOCUMENT_EXISTS"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "EXECUTION_INTERRUPTED"
  | "EXECUTOR_MISMATCH"
  | "IO_FAILURE"
  | "RESULT_MISMATCH"
  | "RESOURCE_CONFLICT"
  | "RESOURCE_INCOMPLETE"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RENDER_FAILURE";

export interface Failure {
  code: FailureCode;
  detail?: unknown;
  message: string;
}
