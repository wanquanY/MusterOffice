// Deliberately exercises upstream retry behavior WITHOUT MusterOffice quarantine.
// This is a counterexample observer, never a runtime entry or recovery strategy.
#include "hb.h"
#include "hb-ot.h"
#include "mo_hb_allocator.h"
#include <fstream>
#include <iostream>
#include <vector>
#include <limits>
#include <cstdlib>
#include <string>
static void observe(const std::vector<char> &data) {
  hb_blob_t *blob=hb_blob_create(data.data(),data.size(),HB_MEMORY_MODE_READONLY,nullptr,nullptr);
  hb_face_t *face=hb_face_create(blob,0);
  (void)hb_face_get_glyph_count(face);
  hb_font_t *font=hb_font_create(face);
  hb_ot_font_set_funcs(font);
  hb_font_set_scale(font,64000,64000);
  hb_font_set_variations(font,nullptr,0);
  hb_buffer_t *buffer=hb_buffer_create();
  hb_buffer_set_direction(buffer,HB_DIRECTION_LTR);
  hb_buffer_set_script(buffer,HB_SCRIPT_LATIN);
  hb_buffer_set_language(buffer,hb_language_from_string("en",2));
  hb_buffer_set_flags(buffer,static_cast<hb_buffer_flags_t>(67));
  const std::u32string text=U"office affine AV a\u0301";
  std::vector<uint32_t> scalars(text.begin(),text.end());
  hb_buffer_add_utf32(buffer,scalars.data(),scalars.size(),0,scalars.size());
  const char *shapers[]={"ot",nullptr};
  const bool shaped=hb_shape_full(font,buffer,nullptr,0,shapers);
  unsigned count=0;auto *glyphs=hb_buffer_get_glyph_infos(buffer,&count);
  std::cout<<"{\"shapeReturn\":"<<(shaped?"true":"false")<<",\"bufferSuccessful\":"<<(hb_buffer_allocation_successful(buffer)?"true":"false")
           <<",\"allocationFailed\":"<<(mo_hb_alloc_failed()?"true":"false")<<",\"glyphs\":[";
  for(unsigned i=0;i<count;++i){if(i)std::cout<<",";std::cout<<glyphs[i].codepoint;}
  std::cout<<"]}";
  hb_buffer_destroy(buffer);hb_font_destroy(font);hb_face_destroy(face);hb_blob_destroy(blob);
}
int main(int argc,char **argv) {
  if(argc!=3)return 1;
  std::ifstream file(argv[1],std::ios::binary|std::ios::ate);if(!file)return 2;
  auto size=file.tellg();if(size<0 || size>128*1024*1024)return 3;
  std::vector<char> data(size);file.seekg(0);file.read(data.data(),size);if(!file)return 4;
  mo_hb_fail_after(std::strtoull(argv[2],nullptr,10));mo_hb_alloc_begin();
  std::cout<<"{\"first\":";observe(data);
  mo_hb_fail_after(std::numeric_limits<size_t>::max());mo_hb_alloc_begin();
  std::cout<<",\"retry\":";observe(data);std::cout<<"}"<<std::endl;
}
