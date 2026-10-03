/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "CellId".
 */
export type CellId = string;
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "TextEditAvailability".
 */
export type TextEditAvailability =
  | {
      kind: "available";
    }
  | {
      kind: "unavailable";
      reason: TextEditRestriction;
    };
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "TextEditRestriction".
 */
export type TextEditRestriction =
  | {
      kind: "unsupportedTarget";
    }
  | {
      kind: "coveredCell";
    }
  | {
      kind: "missingTextBody";
    }
  | {
      kind: "textBodyExists";
    }
  | {
      kind: "selectionRequired";
    }
  | {
      kind: "nonemptySelectionRequired";
    }
  | {
      kind: "nativeStructureRequired";
    }
  | {
      kind: "retainedParagraphBoundary";
    }
  | {
      kind: "nativeInsertionTarget";
    }
  | {
      constraint: NativeEditConstraint;
      kind: "nativeRun";
      run: RunId;
    }
  | {
      kind: "structuredRun";
      run: RunId;
      runKind: RetainedRunKind;
    };
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "NativeEditConstraint".
 */
export type NativeEditConstraint =
  | "missingDirectTransform"
  | "retainedTransform"
  | "compatibilityBranch"
  | "structuredLeaf"
  | "dynamicField"
  | "timingReferences"
  | "retainedReferences";
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "RunId".
 */
export type RunId = string;
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "RetainedRunKind".
 */
export type RetainedRunKind = "text" | "break" | "field";
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "TextReplacementPolicy".
 */
export type TextReplacementPolicy = "authoredBody" | "retainedTextLeaves";
/**
 * This interface was referenced by `TextEditingCapabilities`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

/**
 * Model/selection prerequisites only, not host authorization, font availability
 * or a promise that arbitrary replacement text passes document limits.
 */
export interface TextEditingCapabilities {
  cell?: CellId | null;
  characterStyle: TextEditAvailability;
  /**
   * Deletion has no insertion-style owner; it can be legal beside a protected run.
   */
  delete:
    | {
        kind: "available";
      }
    | {
        kind: "unavailable";
        reason: TextEditRestriction;
      };
  documentId: DocumentId;
  initialize: TextEditAvailability;
  object: ObjectId;
  replace: TextEditAvailability;
  replacementPolicy?: TextReplacementPolicy | null;
  revision: Digest;
}
