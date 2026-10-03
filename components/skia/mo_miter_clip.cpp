#include "mo_miter_clip.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPathBuilder.h"
#include "src/core/SkStroke.h"
#include <algorithm>
#include <cmath>
#include <utility>

namespace {
// This callback runs at the actual segment joins, including the closing join.
// Curve subdivision, offset curves, caps and winding remain owned by Skia.
// The supplied limit is reciprocal, as in SkStrokerPriv::JoinProc.
void clipped_join(SkPathBuilder* outer, SkPathBuilder* inner,
                  const SkVector& before, const SkPoint& pivot,
                  const SkVector& after, SkScalar radius, SkScalar inverse_limit,
                  bool /* previous_is_line */, bool /* current_is_line */) {
    double bx = before.x(), by = before.y(), ax = after.x(), ay = after.y();
    const double cross = bx * ay - by * ax;
    // The exact reversal has no unique bisector. Its defined join is bevel;
    // straight continuations have coincident offset endpoints. No angle epsilon
    // converts almost-reversals into bevels.
    if (cross < 0) {
        std::swap(outer, inner);
        bx = -bx; by = -by; ax = -ax; ay = -ay;
    }
    const double r = radius;
    auto point = [&](double x, double y) {
        return SkPoint::Make(float(double(pivot.x()) + x), float(double(pivot.y()) + y));
    };
    if (cross != 0) {
        const double bn = std::hypot(bx, by), an = std::hypot(ax, ay);
        bx /= bn; by /= bn; ax /= an; ay /= an;
        // Rotate the normal difference for sharp corners: unlike their sum it
        // remains well-conditioned as the tangents approach an exact reversal.
        double mx, my;
        if (bx * ax + by * ay < 0) {
            mx = ay - by; my = bx - ax;
            if (cross < 0) { mx = -mx; my = -my; }
        } else {
            mx = bx + ax; my = by + ay;
        }
        const double norm = std::hypot(mx, my);
        mx /= norm; my /= norm;
        const double c = std::clamp(bx * mx + by * my, 0.0, 1.0);
        const double limit = 1.0 / double(inverse_limit);
        if (c * limit >= 1) {
            outer->lineTo(point(r * mx / c, r * my / c));
        } else {
            // Intersect the two offset tangent lines with dot(q,m)=r*limit.
            // The cross component stays nonzero in this branch (limit>=1).
            const double s = std::abs(bx * my - by * mx);
            const double along = r * limit;
            const double half = (cross > 0 ? r : -r) * (1 - limit * c) / s;
            outer->lineTo(point(along * mx + half * my, along * my - half * mx));
            outer->lineTo(point(along * mx - half * my, along * my + half * mx));
        }
    }
    // Keep each segment's true offset endpoint. Replacing it with a clip corner
    // would incorrectly alter short adjacent segments and curved tangents.
    outer->lineTo(point(r * ax, r * ay));
    inner->lineTo(pivot);
    inner->lineTo(point(-r * ax, -r * ay));
}
}

SkPath mo_stroke_outline(const SkPath& path, const SkPaint& paint, bool clipped_miter) {
    const float width = paint.getStrokeWidth();
    SkStroke stroke(paint, width == 0 ? 1.0f : width);
    SkPathBuilder outline;
    if (clipped_miter) stroke.strokePathWithJoiner(path, &outline, clipped_join);
    else stroke.strokePath(path, &outline);
    return outline.detach();
}
void mo_draw_miter_clip(SkCanvas& canvas, const SkPath& path, const SkPaint& paint) {
    SkPaint fill = paint;
    fill.setStyle(SkPaint::kFill_Style);
    canvas.drawPath(mo_stroke_outline(path, paint, true), fill);
}
