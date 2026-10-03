#include "mo_hb_caret_points.h"
#include <cmath>
#include <cstring>

namespace {
constexpr uint32_t magic = 0x4d4f4354;
constexpr unsigned max_glyphs = 256, max_carets = 64;

int32_t carets(const uint8_t *bytes, uint32_t length, const uint32_t *r, uint32_t words,
               uint32_t **output, uint32_t *output_words) {
  if (!output || !output_words || !bytes || !length || length > 128*1024*1024 || !r || words < 8) return 1;
  if (r[0] != magic || r[1] != 1 || r[3] > 64 || r[4] < 4 || r[4] > 7 ||
      r[5] > max_glyphs || r[6] != max_carets || r[7] != 0 || words != 8+r[3]*2+r[5]) return 1;
  hb_variation_t variations[64];
  for (uint32_t i = 0; i < r[3]; ++i) {
    const auto *v = r+8+i*2; float value; std::memcpy(&value, v+1, sizeof value);
    if (!mo_hb::tag(v[0]) || !std::isfinite(value)) return 1;
    for (uint32_t j = 0; j < i; ++j) if (variations[j].tag == v[0]) return 1;
    variations[i] = {v[0], value};
  }
  const auto *glyphs = r+8+r[3]*2;
  mo_hb::FontInstance instance;
  const auto status = instance.load(bytes, length, r[2], variations, r[3]);
  if (status) return status;
  mo_hb::CaretFont caret_font(instance.font.value, static_cast<hb_direction_t>(r[4]));
  const auto caret_status = caret_font.load();
  if (caret_status) return caret_status;
  unsigned counts[max_glyphs]; uint32_t total_words = 6;
  const auto direction = static_cast<hb_direction_t>(r[4]);
  for (uint32_t i = 0; i < r[5]; ++i) {
    if (glyphs[i] >= hb_face_get_glyph_count(instance.face.value)) return 1;
    for (uint32_t j = 0; j < i; ++j) if (glyphs[j] == glyphs[i]) return 1;
    counts[i] = hb_ot_layout_get_ligature_carets(caret_font.font.value, direction, glyphs[i], 0, nullptr, nullptr);
    if (counts[i] > max_carets) return 4;
    total_words += 2+counts[i];
  }
  auto *result = static_cast<uint32_t *>(mo_hb_alloc(total_words*sizeof(uint32_t)));
  if (!result) return 2;
  result[0] = magic; result[1] = 1; result[2] = instance.upem;
  result[3] = instance.upem*64; result[4] = r[4]; result[5] = r[5];
  uint32_t offset = 6;
  for (uint32_t i = 0; i < r[5]; ++i) {
    hb_position_t positions[max_carets]; unsigned count = counts[i];
    const auto total = hb_ot_layout_get_ligature_carets(caret_font.font.value, direction, glyphs[i], 0, &count, positions);
    if (caret_font.status) { mo_hb_free(result); return caret_font.status; }
    if (total != counts[i] || count != counts[i]) { mo_hb_free(result); return 3; }
    result[offset++] = glyphs[i]; result[offset++] = count;
    for (unsigned j = 0; j < count; ++j) result[offset++] = static_cast<uint32_t>(positions[j]);
  }
  *output = result; *output_words = total_words;
  return 0;
}
}

extern "C" int32_t mo_hb_caret_font(const uint8_t *bytes, uint32_t length, const uint32_t *request,
                                  uint32_t words, uint32_t **output, uint32_t *output_words) {
  return mo_hb::execute([&] { return carets(bytes,length,request,words,output,output_words); },output,output_words);
}
