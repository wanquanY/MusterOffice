#pragma once
// Private to the pinned HarfBuzz 14.5.0 component. Its default OT font functions
// omit contour-point lookup. Reuse its exact glyf/gvar evaluator, preserving
// original point indices (drawing callbacks reorder/off-curve-expand points).
#include "hb-font.hh"
#include "hb-ot-face.hh"
#include "hb-ot-glyf-table.hh"
#include "mo_hb_font.h"
#include <cmath>
#include <limits>
namespace mo_hb {
struct CaretFont {
  hb_font_t *base;
  hb_direction_t direction;
  hb_glyf_scratch_t scratch;
  contour_point_vector_t points;
  hb_codepoint_t cached = HB_CODEPOINT_INVALID;
  int64_t budget = HB_BUDGET_GLYPH; // Shared by every queried glyph in this instance.
  int32_t status = 0;
  Owned<hb_font_funcs_t,hb_font_funcs_destroy> funcs;
  Owned<hb_font_t,hb_font_destroy> font;
  explicit CaretFont(hb_font_t *parent, hb_direction_t dir) : base(parent), direction(dir) {}
  static hb_bool_t point(hb_font_t *, void *data, hb_codepoint_t glyph,
                         unsigned index, hb_position_t *x, hb_position_t *y, void *) {
    auto &self = *static_cast<CaretFont *>(data);
    if (self.status) return false;
    auto *glyf = self.base->face->table.glyf.operator->();
    if (!glyf->has_data()) { self.status = 3; return false; }
    if (self.cached != glyph) {
      self.points.clear();
      if (!glyf->glyph_for_gid(glyph).get_points(self.base, *glyf, self.points,
          self.scratch, nullptr, nullptr, nullptr, true, true, false,
          hb_array(self.base->coords, self.base->has_nonzero_coords ? self.base->num_coords : 0),
          nullptr, 0, nullptr, &self.budget)) {
        self.status = mo_hb_alloc_failed() ? 2 : (self.budget < 0 ? 4 : 3); return false;
      }
      self.cached = glyph;
    }
    if (self.points.length < OT::glyf_impl::PHANTOM_COUNT ||
        index >= self.points.length - OT::glyf_impl::PHANTOM_COUNT) {
      self.status = 3; return false;
    }
    const auto &p = self.points[index];
    const double sx = std::round(static_cast<double>(self.base->em_fscalef_x(p.x)));
    const double sy = std::round(static_cast<double>(self.base->em_fscalef_y(p.y)));
    const auto fits = [](double v) { return std::isfinite(v) &&
        v >= std::numeric_limits<int32_t>::min() && v <= std::numeric_limits<int32_t>::max(); };
    if (!fits(sx) || !fits(sy)) { self.status = 3; return false; }
    hb_position_t ox = 0, oy = 0;
    self.base->get_glyph_origin_for_direction(glyph, self.direction, &ox, &oy);
    if (!fits(sx - ox) || !fits(sy - oy)) { self.status = 3; return false; }
    *x = static_cast<int32_t>(sx);
    *y = static_cast<int32_t>(sy);
    return true;
  }
  int32_t load() {
    funcs.value = hb_font_funcs_create();
    font.value = hb_font_create_sub_font(base);
    if (funcs.value == hb_font_funcs_get_empty() || font.value == hb_font_get_empty()) return 2;
    hb_font_funcs_set_glyph_contour_point_func(funcs.value, point, nullptr, nullptr);
    hb_font_funcs_make_immutable(funcs.value);
    hb_font_set_funcs(font.value, funcs.value, this, nullptr);
    return mo_hb_alloc_failed() ? 2 : 0;
  }
};
}
