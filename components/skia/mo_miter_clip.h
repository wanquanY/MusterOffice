#pragma once

class SkCanvas;
class SkPaint;
class SkPath;

// Evaluated device geometry only. Does not interpret DrawingML or application
// compatibility rules. Width zero is an explicit one-device-pixel outline.
void mo_draw_miter_clip(SkCanvas&, const SkPath&, const SkPaint&);
