#pragma once
// Private shared construction and failure ownership for shaping and measurement.
#include "mo_hb.h"
#include "mo_hb_allocator.h"
#include "hb.h"
#include "hb-ot.h"
namespace mo_hb {
template <typename T, void (*Destroy)(T *)> struct Owned {
  T *value;
  explicit Owned(T *p=nullptr) : value(p) {}
  ~Owned() { if (value) Destroy(value); }
  Owned(const Owned &) = delete;
  Owned &operator=(const Owned &) = delete;
};
inline bool tag(uint32_t value) {
  for (unsigned shift=0;shift<32;shift+=8) {
    unsigned byte=(value>>shift)&255;
    if (byte<32 || byte>126) return false;
  }
  return true;
}
struct FontInstance {
  Owned<hb_blob_t,hb_blob_destroy> blob;
  Owned<hb_face_t,hb_face_destroy> face;
  Owned<hb_font_t,hb_font_destroy> font;
  unsigned upem=0;
  int32_t load(const uint8_t *bytes,uint32_t length,uint32_t index,
               const hb_variation_t *variations,unsigned count) {
    blob.value=hb_blob_create(reinterpret_cast<const char *>(bytes),length,HB_MEMORY_MODE_READONLY,nullptr,nullptr);
    if (hb_blob_get_length(blob.value)!=length) return 2;
    if (index>=hb_face_count(blob.value)) return 5;
    face.value=hb_face_create(blob.value,index);
    if (face.value==hb_face_get_empty() || !hb_face_get_glyph_count(face.value)) return 5;
    upem=hb_face_get_upem(face.value);
    if (upem<16 || upem>16384) return 5;
    font.value=hb_font_create(face.value);
    if (font.value==hb_font_get_empty()) return 2;
    hb_ot_font_set_funcs(font.value);
    hb_font_set_scale(font.value,upem*64,upem*64);
    for (unsigned i=0;i<count;++i) {
      hb_ot_var_axis_info_t axis;
      if (!hb_ot_var_find_axis_info(face.value,variations[i].tag,&axis)
          || variations[i].value<axis.min_value || variations[i].value>axis.max_value) return 1;
    }
    hb_font_set_variations(font.value,variations,count);
    return mo_hb_alloc_failed() ? 2 : 0;
  }
};
template <typename Compute> int32_t execute(Compute compute,uint32_t **output,uint32_t *words) {
  if (output) *output=nullptr;
  if (words) *words=0;
  if (mo_hb_instance_invalid()) return 6;
  mo_hb_alloc_begin();
  const int32_t status=compute();
  if (status==2) mo_hb_invalidate();
  if (mo_hb_alloc_failed() || mo_hb_instance_invalid()) {
    if (output && *output) mo_hb_free(*output);
    if (output) *output=nullptr;
    if (words) *words=0;
    return (status==2 || mo_hb_alloc_failed()) ? 2 : 6;
  }
  return status;
}
}
