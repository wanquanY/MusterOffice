/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type AuthoringAction =
  | {
      kind: "create";
      pageSize: Size;
      slides: SlideContent[];
      title: string;
    }
  | {
      kind: "append";
      slides: SlideContent[];
    }
  | {
      edits: ContentEdit[];
      kind: "edit";
    };
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type Fill =
  | {
      kind: "none";
    }
  | {
      color: Color;
      kind: "solid";
    };
export type Color =
  | {
      kind: "srgb";
      rgba: Rgba;
    }
  | {
      kind: "theme";
      slot: ThemeColor;
    };
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
 * Ordered native shapes and pictures; legacy shape declarations remain valid.
 */
export type ElementContent = PictureContent | ShapeContent;
export type ObjectId = string;
export type ResourceId = string;
export type PathCommand =
  | {
      kind: "move";
      to: Point;
    }
  | {
      kind: "line";
      to: Point;
    }
  | {
      control: Point;
      kind: "quadratic";
      to: Point;
    }
  | {
      control1: Point;
      control2: Point;
      kind: "cubic";
      to: Point;
    }
  | {
      kind: "close";
    };
export type Stroke =
  | {
      kind: "none";
    }
  | {
      /**
       * Absent retains an unresolved declaration, not an implicit flat cap.
       */
      cap?: LineCap | null;
      color: Color;
      /**
       * Absent retains the source/default distinction.
       */
      join?: LineJoin | null;
      kind: "solid";
      width: Emu;
    };
export type LineCap = "flat" | "round" | "square";
export type LineJoin =
  | {
      kind: "round";
    }
  | {
      kind: "bevel";
    }
  | {
      kind: "miter";
      /**
       * Ratio in 1/100000 units: 400000 denotes four times line width.
       * The ratio compares full miter length with the full stroke width.
       */
      limit?: number | null;
    };
export type Alignment = "start" | "center" | "end" | "justify";
export type TextDirection = "leftToRight" | "rightToLeft" | "verticalRightToLeft" | "verticalLeftToRight";
/**
 * Native paragraph line spacing. Percentage is measured against the line's
 * largest font size by the shared layout engine; 100000 means 100%, 150000 150%.
 */
export type ParagraphLineSpacing =
  | {
      kind: "percent";
      value: number;
    }
  | {
      height: Emu;
      kind: "exact";
    };
export type SlideId = string;
/**
 * Targeted edits preserve object/slide identities and unrelated properties.
 * Replacing text is explicit: use the same PlainText contract as creation.
 * Deletions reject dependencies by default; no implicit cascade or page rebuild.
 */
export type ContentEdit =
  | {
      kind: "setTitle";
      title: string;
    }
  | {
      kind: "setSlideName";
      name: string;
      slide: SlideId;
    }
  | {
      background: Inherited;
      kind: "setSlideBackground";
      slide: SlideId;
    }
  | {
      element: ElementContent;
      index: number;
      kind: "insertElement";
      slide: SlideId;
    }
  | {
      fill: Inherited;
      kind: "setFill";
      object: ObjectId;
    }
  | {
      kind: "setStroke";
      object: ObjectId;
      stroke: Inherited2;
    }
  | {
      geometry: Geometry;
      kind: "setGeometry";
      object: ObjectId;
    }
  | {
      crop?: Crop1 | null;
      kind: "replacePicture";
      object: ObjectId;
      resource: ResourceId;
    }
  | {
      crop: Crop1;
      kind: "setPictureCrop";
      object: ObjectId;
    }
  | {
      frame: ShapeFrame;
      kind: "setFrame";
      object: ObjectId;
    }
  | {
      kind: "replaceText";
      object: ObjectId;
      text: PlainText;
    }
  | {
      accessibility: Accessibility2;
      kind: "setAccessibility";
      object: ObjectId;
    }
  | {
      index: number;
      kind: "moveSlide";
      slide: SlideId;
    }
  | {
      kind: "deleteSlide";
      slide: SlideId;
    }
  | {
      kind: "deleteObject";
      object: ObjectId;
    };
export type Inherited =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Fill;
    };
export type Inherited2 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Stroke;
    };
export type Geometry =
  | {
      kind: "rectangle";
    }
  | {
      kind: "ellipse";
    }
  | {
      kind: "roundRectangle";
      radius: Emu;
    }
  | {
      commands: PathCommand[];
      kind: "path";
      viewport: Size;
    };

export interface Size {
  height: Emu;
  width: Emu;
}
export interface SlideContent {
  /**
   * Omitted background inherits the same caller delivery defaults as Document.
   */
  background?: Fill | null;
  /**
   * Array order is paint order, back to front.
   */
  elements: ElementContent[];
  id: SlideId;
  name?: string;
}
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}
export interface PictureContent {
  accessibility?: Accessibility;
  frame: ShapeFrame;
  id: ObjectId;
  picture: PictureSource;
}
export interface Accessibility {
  decorative: boolean;
  description?: string;
  title?: string;
}
export interface ShapeFrame {
  flipHorizontal?: boolean;
  flipVertical?: boolean;
  height: Emu;
  /**
   * DrawingML units: 60000 per degree, just as the native model.
   */
  rotation?: number;
  width: Emu;
  x: Emu;
  y: Emu;
}
export interface PictureSource {
  crop?: Crop;
  resource: ResourceId;
}
/**
 * Native crop fractions: 100000 is the full source extent.
 */
export interface Crop {
  bottom: number;
  left: number;
  right: number;
  top: number;
}
export interface ShapeContent {
  accessibility?: Accessibility1;
  /**
   * Omitted fill/stroke mean explicit none, not implicit theme paint.
   */
  fill?: Fill | null;
  frame: ShapeFrame;
  /**
   * Omitted geometry is a rectangle; text remains native editable text.
   */
  geometry?:
    | {
        kind: "rectangle";
      }
    | {
        kind: "ellipse";
      }
    | {
        kind: "roundRectangle";
        radius: Emu;
      }
    | {
        commands: PathCommand[];
        kind: "path";
        viewport: Size;
      };
  id: ObjectId;
  stroke?: Stroke | null;
  text?: PlainText | null;
}
export interface Accessibility1 {
  decorative: boolean;
  description?: string;
  title?: string;
}
export interface Point {
  x: Emu;
  y: Emu;
}
export interface PlainText {
  insets?: Insets;
  style?: PlainTextStyle;
  /**
   * LF, CRLF and CR each separate paragraphs; tabs become native tab runs.
   * Empty paragraphs and a trailing paragraph separator are preserved.
   */
  text: string;
  wrap?: boolean;
}
export interface Insets {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}
/**
 * Omitted properties inherit explicit delivery defaults. Font selection
 * uses the host's explicit default; use full Document creation for custom
 * font resources. This operation never searches installed system fonts.
 */
export interface PlainTextStyle {
  alignment?: Alignment | null;
  bold?: boolean | null;
  color?: Color | null;
  direction?: TextDirection | null;
  indent?: Emu | null;
  italic?: boolean | null;
  language?: string | null;
  leftMargin?: Emu | null;
  lineSpacing?: ParagraphLineSpacing | null;
  rightMargin?: Emu | null;
  size?: Emu | null;
  spaceAfter?: Emu | null;
  spaceBefore?: Emu | null;
  underline?: boolean | null;
}
export interface Crop1 {
  bottom: number;
  left: number;
  right: number;
  top: number;
}
export interface Accessibility2 {
  decorative: boolean;
  description?: string;
  title?: string;
}
