#pragma once

class SkCanvas;
class SkPaint;
class SkPath;

// Evaluated device geometry only. Does not interpret DrawingML or application
// compatibility rules. Width zero is an explicit one-device-pixel outline.
void mo_draw_miter_clip(SkCanvas&, const SkPath&, const SkPaint&);

// The same native stroke geometry is also used for point picking.
SkPath mo_stroke_outline(const SkPath&, const SkPaint&, bool clipped_miter);
