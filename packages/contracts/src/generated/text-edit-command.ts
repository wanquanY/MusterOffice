/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "TextEditAction".
 */
export type TextEditAction =
  | {
      kind: "replace";
      selection: TextSelection;
      text: string;
    }
  | {
      kind: "setCharacterStyle";
      patch: CharacterStylePatch;
      selection: TextSelection;
    };
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Affinity".
 */
export type Affinity = "before" | "after";
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Inherited4".
 */
export type Inherited4 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: boolean;
    };
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Inherited3".
 */
export type Inherited3 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Color;
    };
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Color".
 */
export type Color =
  | {
      kind: "srgb";
      rgba: Rgba;
    }
  | {
      kind: "theme";
      slot: ThemeColor;
    };
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "ThemeColor".
 */
export type ThemeColor =
  | "dark1"
  | "light1"
  | "dark2"
  | "light2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hyperlink"
  | "followedHyperlink";
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Inherited".
 */
export type Inherited =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: FontId;
    };
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Inherited5".
 */
export type Inherited5 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: string;
    };
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Inherited2".
 */
export type Inherited2 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Emu;
    };
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 *
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Emu".
 */
export type Emu = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "OperationId".
 */
export type OperationId = string;
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;

export interface TextEditCommand {
  action: TextEditAction;
  baseRevision: Digest;
  documentId: DocumentId;
  object: ObjectId;
  operationId: OperationId;
  requestId: RequestId;
}
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "TextSelection".
 */
export interface TextSelection {
  anchor: TextAnchor;
  focus: TextAnchor;
}
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "TextAnchor".
 */
export interface TextAnchor {
  affinity: Affinity;
  paragraph: ParagraphId;
  scalarOffset: number;
}
/**
 * Omitted fields keep their declarations; `inherit` explicitly resets one
 * field. Applying bold must not resolve or overwrite font/theme inheritance.
 *
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "CharacterStylePatch".
 */
export interface CharacterStylePatch {
  bold?: Inherited4 | null;
  color?: Inherited3 | null;
  font?: Inherited | null;
  italic?: Inherited4 | null;
  language?: Inherited5 | null;
  size?: Inherited2 | null;
  underline?: Inherited4 | null;
}
/**
 * This interface was referenced by `TextEditCommand`'s JSON-Schema
 * via the `definition` "Rgba".
 */
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}
