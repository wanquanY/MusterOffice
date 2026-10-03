/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `TextCapabilitiesQuery`'s JSON-Schema
 * via the `definition` "CellId".
 */
export type CellId = string;
/**
 * This interface was referenced by `TextCapabilitiesQuery`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `TextCapabilitiesQuery`'s JSON-Schema
 * via the `definition` "Affinity".
 */
export type Affinity = "before" | "after";
/**
 * This interface was referenced by `TextCapabilitiesQuery`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;

export interface TextCapabilitiesQuery {
  cell?: CellId | null;
  object: ObjectId;
  selection?: TextSelection | null;
}
/**
 * This interface was referenced by `TextCapabilitiesQuery`'s JSON-Schema
 * via the `definition` "TextSelection".
 */
export interface TextSelection {
  anchor: TextAnchor;
  focus: TextAnchor;
}
/**
 * This interface was referenced by `TextCapabilitiesQuery`'s JSON-Schema
 * via the `definition` "TextAnchor".
 */
export interface TextAnchor {
  affinity: Affinity;
  paragraph: ParagraphId;
  scalarOffset: number;
}
