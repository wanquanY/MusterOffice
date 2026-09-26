#include "mo_hb_font.h"
#include "mo_hb_allocator.h"
#include "hb.h"
#include "hb-ot.h"
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <limits>

namespace {
constexpr uint32_t magic = 0x4d4f4842;
constexpr uint32_t header = 13;
constexpr uint32_t maximum_text = 65536;
constexpr uint32_t maximum_features = 1024;
constexpr uint32_t maximum_variations = 64;
constexpr uint32_t maximum_glyphs = 262144;
// VERIFY can write stderr/change output on failed verification; it is not a
// production rendering option. Dedicated comparison tests own verification.
constexpr uint32_t allowed_flags = 0xff & ~HB_BUFFER_FLAG_VERIFY;
using mo_hb::Owned;
using mo_hb::tag;
bool scalar(uint32_t cp) { return cp <= 0x10ffff && !(cp >= 0xd800 && cp <= 0xdfff); }
}

extern "C" uint32_t mo_hb_version(void) {
  return (HB_VERSION_MAJOR << 16) | (HB_VERSION_MINOR << 8) | HB_VERSION_MICRO;
}
extern "C" void mo_hb_free(void *allocation) { mo_hb_dealloc(allocation); }
static int32_t shape_impl(const uint8_t *bytes, uint32_t byte_length,
                                const uint32_t *r, uint32_t words,
                                const char *language, uint32_t language_length,
                                uint32_t **output, uint32_t *output_words) {
  if (!output || !output_words) return 1;
  *output = nullptr; *output_words = 0;
  if (!r || words < header || !bytes || byte_length == 0 || byte_length > 128*1024*1024
      || !language || language_length == 0 || language_length > 255) return 1;
  if (r[0] != magic || r[12] != 1 || r[2] < 4 || r[2] > 7 || !tag(r[3])
      || (r[4] & ~allowed_flags) || ((r[4] & 12) == 12) || r[5] > 3
      || r[6] > maximum_text || r[7] > r[6] || r[8] > r[6]-r[7]
      || r[9] > maximum_features || r[10] > maximum_variations
      || r[11] > maximum_glyphs) return 1;
  const uint64_t needed = uint64_t(header)+r[6]+uint64_t(r[9])*4+uint64_t(r[10])*2;
  if (needed != words) return 1;
  for (uint32_t i=0; i<language_length; ++i) {
    const unsigned char c = language[i];
    if (!((c>='a' && c<='z') || (c>='A' && c<='Z') || (c>='0' && c<='9') || c=='-')) return 1;
  }
  const uint32_t *text = r+header;
  for (uint32_t i=0; i<r[6]; ++i) if (!scalar(text[i])) return 1;
  hb_feature_t features[maximum_features];
  const uint32_t *f = text+r[6];
  for (uint32_t i=0; i<r[9]; ++i, f+=4) {
    if (!tag(f[0]) || f[2]>f[3]) return 1;
    features[i] = {f[0],f[1],f[2],f[3]};
  }
  hb_variation_t variations[maximum_variations];
  for (uint32_t i=0; i<r[10]; ++i, f+=2) {
    if (!tag(f[0])) return 1;
    float value; std::memcpy(&value,f+1,sizeof value);
    if (!std::isfinite(value)) return 1;
    for (uint32_t j=0; j<i; ++j) if (variations[j].tag==f[0]) return 1;
    variations[i] = {f[0],value};
  }
  mo_hb::FontInstance instance;
  const auto font_status=instance.load(bytes,byte_length,r[1],variations,r[10]);
  if (font_status) return font_status;
  auto &font=instance.font;
  const unsigned upem=instance.upem;
  Owned<hb_buffer_t,hb_buffer_destroy> buffer(hb_buffer_create());
  if (buffer.value==hb_buffer_get_empty()) return 2;
  hb_buffer_set_direction(buffer.value,static_cast<hb_direction_t>(r[2]));
  hb_buffer_set_script(buffer.value,hb_script_from_iso15924_tag(r[3]));
  const hb_language_t lang=hb_language_from_string(language,language_length);
  if (lang==HB_LANGUAGE_INVALID) return 2;
  hb_buffer_set_language(buffer.value,lang);
  hb_buffer_set_cluster_level(buffer.value,static_cast<hb_buffer_cluster_level_t>(r[5]));
  hb_buffer_set_flags(buffer.value,static_cast<hb_buffer_flags_t>(r[4]));
  hb_buffer_add_utf32(buffer.value,text,r[6],r[7],r[8]);
  if (!hb_buffer_allocation_successful(buffer.value)) return 2;
  const char *shapers[] = {"ot",nullptr};
  const bool shaped=hb_shape_full(font.value,buffer.value,features,r[9],shapers);
  if (!hb_buffer_allocation_successful(buffer.value)) return 2;
  if (!shaped) return 3;
  unsigned n=0, positions_count=0;
  const hb_glyph_info_t *infos=hb_buffer_get_glyph_infos(buffer.value,&n);
  const hb_glyph_position_t *positions=hb_buffer_get_glyph_positions(buffer.value,&positions_count);
  if (!hb_buffer_allocation_successful(buffer.value)) return 2;
  if (n!=positions_count || (n && (!infos || !positions))
      || (n && hb_buffer_get_content_type(buffer.value)!=HB_BUFFER_CONTENT_TYPE_GLYPHS)) return 3;
  if (mo_hb_alloc_failed()) return 2;
  if (n>r[11]) return 4;
  const uint32_t length=8+n*7;
  auto *result=static_cast<uint32_t *>(mo_hb_alloc(length*sizeof(uint32_t)));
  if (!result) return 2;
  result[0]=magic; result[1]=1; result[2]=upem; result[3]=upem*64;
  result[4]=n; result[5]=hb_buffer_get_flags(buffer.value); result[6]=r[5]; result[7]=0;
  for (unsigned i=0; i<n; ++i) {
    auto *out=result+8+i*7;
    out[0]=infos[i].codepoint; out[1]=infos[i].cluster; out[2]=hb_glyph_info_get_glyph_flags(infos+i);
    out[3]=static_cast<uint32_t>(positions[i].x_advance); out[4]=static_cast<uint32_t>(positions[i].y_advance);
    out[5]=static_cast<uint32_t>(positions[i].x_offset); out[6]=static_cast<uint32_t>(positions[i].y_offset);
  }
  *output=result; *output_words=length;
  return 0;
}

extern "C" int32_t mo_hb_shape(const uint8_t *bytes, uint32_t byte_length,
                                const uint32_t *request, uint32_t words,
                                const char *language, uint32_t language_length,
                                uint32_t **output, uint32_t *output_words) {
  return mo_hb::execute([&]{return shape_impl(bytes,byte_length,request,words,language,language_length,output,output_words);},output,output_words);
}
