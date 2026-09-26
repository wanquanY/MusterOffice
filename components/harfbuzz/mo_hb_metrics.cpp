#include "mo_hb_font.h"
#include <cmath>
#include <cstring>
namespace {
constexpr uint32_t magic=0x4d4f4d54;
constexpr hb_ot_metrics_tag_t metrics[]={
  HB_OT_METRICS_TAG_HORIZONTAL_ASCENDER,HB_OT_METRICS_TAG_HORIZONTAL_DESCENDER,HB_OT_METRICS_TAG_HORIZONTAL_LINE_GAP,
  HB_OT_METRICS_TAG_HORIZONTAL_CLIPPING_ASCENT,HB_OT_METRICS_TAG_HORIZONTAL_CLIPPING_DESCENT,
  HB_OT_METRICS_TAG_VERTICAL_ASCENDER,HB_OT_METRICS_TAG_VERTICAL_DESCENDER,HB_OT_METRICS_TAG_VERTICAL_LINE_GAP,
  HB_OT_METRICS_TAG_HORIZONTAL_CARET_RISE,HB_OT_METRICS_TAG_HORIZONTAL_CARET_RUN,HB_OT_METRICS_TAG_HORIZONTAL_CARET_OFFSET,
  HB_OT_METRICS_TAG_VERTICAL_CARET_RISE,HB_OT_METRICS_TAG_VERTICAL_CARET_RUN,HB_OT_METRICS_TAG_VERTICAL_CARET_OFFSET,
  HB_OT_METRICS_TAG_X_HEIGHT,HB_OT_METRICS_TAG_CAP_HEIGHT,
  HB_OT_METRICS_TAG_SUBSCRIPT_EM_X_SIZE,HB_OT_METRICS_TAG_SUBSCRIPT_EM_Y_SIZE,HB_OT_METRICS_TAG_SUBSCRIPT_EM_X_OFFSET,HB_OT_METRICS_TAG_SUBSCRIPT_EM_Y_OFFSET,
  HB_OT_METRICS_TAG_SUPERSCRIPT_EM_X_SIZE,HB_OT_METRICS_TAG_SUPERSCRIPT_EM_Y_SIZE,HB_OT_METRICS_TAG_SUPERSCRIPT_EM_X_OFFSET,HB_OT_METRICS_TAG_SUPERSCRIPT_EM_Y_OFFSET,
  HB_OT_METRICS_TAG_STRIKEOUT_SIZE,HB_OT_METRICS_TAG_STRIKEOUT_OFFSET,HB_OT_METRICS_TAG_UNDERLINE_SIZE,HB_OT_METRICS_TAG_UNDERLINE_OFFSET,
};
int32_t measure(const uint8_t *bytes,uint32_t length,const uint32_t *r,uint32_t words,uint32_t **output,uint32_t *output_words) {
  if (!output || !output_words || !bytes || !length || length>128*1024*1024 || !r || words<6) return 1;
  if (r[0]!=magic || r[1]!=1 || r[3]>64 || r[4]>28 || r[5]!=0 || words!=6+r[3]*2+r[4]) return 1;
  hb_variation_t variations[64];
  for (uint32_t i=0;i<r[3];++i) {
    const auto *v=r+6+i*2;float value;std::memcpy(&value,v+1,sizeof value);
    if (!mo_hb::tag(v[0]) || !std::isfinite(value)) return 1;
    for (uint32_t j=0;j<i;++j) if (variations[j].tag==v[0]) return 1;
    variations[i]={v[0],value};
  }
  const auto *tags=r+6+r[3]*2;
  for (uint32_t i=0;i<r[4];++i) {
    bool found=false;for (auto metric:metrics) if (tags[i]==static_cast<uint32_t>(metric)) found=true;
    if (!found) return 1;
    for (uint32_t j=0;j<i;++j) if (tags[j]==tags[i]) return 1;
  }
  mo_hb::FontInstance instance;
  auto status=instance.load(bytes,length,r[2],variations,r[3]);if (status) return status;
  const uint32_t count=6+r[4]*3;
  auto *result=static_cast<uint32_t *>(mo_hb_alloc(count*sizeof(uint32_t)));
  if (!result) return 2;
  *output=result;*output_words=count; // The shared executor owns failure cleanup.
  result[0]=magic;result[1]=1;result[2]=instance.upem;result[3]=instance.upem*64;result[4]=r[4];result[5]=0;
  for (uint32_t i=0;i<r[4];++i) {
    hb_position_t value=0;
    const bool available=hb_ot_metrics_get_position(instance.font.value,static_cast<hb_ot_metrics_tag_t>(tags[i]),&value);
    result[6+i*3]=tags[i];result[7+i*3]=available?1:0;result[8+i*3]=available?static_cast<uint32_t>(value):0;
  }
  return 0;
}
}
extern "C" int32_t mo_hb_measure_font(const uint8_t *bytes,uint32_t length,const uint32_t *request,uint32_t words,uint32_t **output,uint32_t *output_words) {
  return mo_hb::execute([&]{return measure(bytes,length,request,words,output,output_words);},output,output_words);
}
