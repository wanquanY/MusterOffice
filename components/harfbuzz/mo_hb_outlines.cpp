#include "mo_hb_font.h"
#include <cmath>
#include <cstring>
#include <limits>
namespace {
constexpr uint32_t magic=0x4d4f4f54;
// Fixed-size records: opcode, then up to three (x,y) pairs. Unused words zero.
// Coordinates are design units * 64, rounded nearest with ties away from zero.
struct Paths {
  uint32_t *data=nullptr;
  uint32_t used=0,capacity=0,maximum=0,commands=0,command_limit=0;
  int32_t status=0;
  int64_t budget=0;
  ~Paths() { mo_hb_free(data); }
  bool reserve(uint32_t count) {
    if (status) return false;
    if (count>maximum-used) {status=4;budget=-1;return false;}
    if (used+count<=capacity) return true;
    uint32_t next=capacity?capacity*2:64;
    if (next<used+count) next=used+count;
    if (next>maximum) next=maximum;
    auto *p=static_cast<uint32_t *>(mo_hb_realloc(data,next*sizeof(uint32_t)));
    if (!p) {status=2;budget=-1;return false;}
    data=p;capacity=next;return true;
  }
  void emit(uint32_t opcode,const float *coordinates,unsigned count) {
    if (status) return;
    if (commands>=command_limit) {status=4;budget=-1;return;}
    uint32_t record[7]={opcode,0,0,0,0,0,0};
    for (unsigned i=0;i<count;++i) {
      const double rounded=std::round(static_cast<double>(coordinates[i]));
      if (!std::isfinite(rounded) || rounded<std::numeric_limits<int32_t>::min()
          || rounded>std::numeric_limits<int32_t>::max()) {status=3;budget=-1;return;}
      record[i+1]=static_cast<uint32_t>(static_cast<int32_t>(rounded));
    }
    if (!reserve(7)) return;
    std::memcpy(data+used,record,sizeof record);used+=7;++commands;
  }
};
void move(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float x,float y,void *) {
  const float c[]={x,y};static_cast<Paths *>(p)->emit(1,c,2);
}
void line(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float x,float y,void *) {
  const float c[]={x,y};static_cast<Paths *>(p)->emit(2,c,2);
}
void quadratic(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float x1,float y1,float x,float y,void *) {
  const float c[]={x1,y1,x,y};static_cast<Paths *>(p)->emit(3,c,4);
}
void cubic(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float x1,float y1,float x2,float y2,float x,float y,void *) {
  const float c[]={x1,y1,x2,y2,x,y};static_cast<Paths *>(p)->emit(4,c,6);
}
void close(hb_draw_funcs_t *,void *p,hb_draw_state_t *,void *) {static_cast<Paths *>(p)->emit(5,nullptr,0);}
int64_t *budget(hb_draw_funcs_t *,void *p,void *) {return &static_cast<Paths *>(p)->budget;}
int32_t outlines(const uint8_t *bytes,uint32_t length,const uint32_t *r,uint32_t words,uint32_t **output,uint32_t *output_words) {
  if (!output || !output_words || !bytes || !length || length>128*1024*1024 || !r || words<8) return 1;
  if (r[0]!=magic || r[1]!=1 || r[3]>64 || r[4]>256 || r[5]>262144
      || !r[6] || r[6]>1048576 || r[7]!=0 || words!=8+r[3]*2+r[4]) return 1;
  hb_variation_t variations[64];
  for (uint32_t i=0;i<r[3];++i) {
    const auto *v=r+8+i*2;float value;std::memcpy(&value,v+1,sizeof value);
    if (!mo_hb::tag(v[0]) || !std::isfinite(value)) return 1;
    for (uint32_t j=0;j<i;++j) if (variations[j].tag==v[0]) return 1;
    variations[i]={v[0],value};
  }
  mo_hb::FontInstance instance;
  auto status=instance.load(bytes,length,r[2],variations,r[3]);if (status) return status;
  const auto *glyphs=r+8+r[3]*2;
  const auto glyph_count=hb_face_get_glyph_count(instance.face.value);
  for (uint32_t i=0;i<r[4];++i) if (glyphs[i]>=glyph_count) return 1;
  mo_hb::Owned<hb_draw_funcs_t,hb_draw_funcs_destroy> funcs(hb_draw_funcs_create());
  if (funcs.value==hb_draw_funcs_get_empty()) return 2;
  hb_draw_funcs_set_move_to_func(funcs.value,move,nullptr,nullptr);
  hb_draw_funcs_set_line_to_func(funcs.value,line,nullptr,nullptr);
  hb_draw_funcs_set_quadratic_to_func(funcs.value,quadratic,nullptr,nullptr);
  hb_draw_funcs_set_cubic_to_func(funcs.value,cubic,nullptr,nullptr);
  hb_draw_funcs_set_close_path_func(funcs.value,close,nullptr,nullptr);
  hb_draw_funcs_set_get_budget_remaining_func(funcs.value,budget,nullptr,nullptr);
  hb_draw_funcs_make_immutable(funcs.value);
  Paths paths;paths.maximum=6+r[4]*3+r[5]*7;paths.command_limit=r[5];paths.budget=r[6];
  if (!paths.reserve(6)) return paths.status;
  const uint32_t header[]={magic,1,instance.upem,instance.upem*64,r[4],0};
  std::memcpy(paths.data,header,sizeof header);paths.used=6;
  for (uint32_t i=0;i<r[4];++i) {
    if (!paths.reserve(3)) return paths.status;
    const auto start=paths.used;paths.used+=3;
    const bool available=hb_font_draw_glyph_or_fail(instance.font.value,glyphs[i],funcs.value,&paths);
    if (paths.status) return paths.status;
    if (paths.budget<0) return 4;
    // The interpreter can emit a prefix then fail. Never publish that prefix.
    if (!available) paths.used=start+3;
    paths.data[start]=glyphs[i];paths.data[start+1]=available?1:0;paths.data[start+2]=(paths.used-start-3)/7;
  }
  *output=paths.data;*output_words=paths.used;paths.data=nullptr;
  return 0;
}
}
extern "C" int32_t mo_hb_outline_font(const uint8_t *bytes,uint32_t length,const uint32_t *request,uint32_t words,uint32_t **output,uint32_t *output_words) {
  return mo_hb::execute([&]{return outlines(bytes,length,request,words,output,output_words);},output,output_words);
}
