// Independent development entry point linked against unmodified upstream HB.
// It deliberately does not include the component ABI or its font setup helper.
#include "hb.h"
#include "hb-ot.h"
#include <iostream>
#include <cstdlib>
#include <cstring>
int main(int argc,char **argv) {
  if (argc<4) return 1;
  auto *blob=hb_blob_create_from_file_or_fail(argv[1]);if (!blob) return 2;
  auto *face=hb_face_create(blob,std::strtoul(argv[2],nullptr,10));
  auto *font=hb_font_create(face);hb_ot_font_set_funcs(font);
  const auto upem=hb_face_get_upem(face);hb_font_set_scale(font,upem*64,upem*64);
  hb_variation_t variations[64];unsigned n=0;
  // All argv after tags are HB's own public axis=value parser inputs.
  for(int i=4;i<argc;++i) {
    if(n==64 || !hb_variation_from_string(argv[i],-1,&variations[n])) return 3;
    ++n;
  }
  hb_font_set_variations(font,variations,n);
  std::cout<<"[";const auto length=std::strlen(argv[3]);
  if(length%4) return 4;
  for(size_t i=0;i<length;i+=4) {
    if(i) std::cout<<",";
    hb_position_t value=0;
    if(hb_ot_metrics_get_position(font,static_cast<hb_ot_metrics_tag_t>(hb_tag_from_string(argv[3]+i,4)),&value)) std::cout<<value;
    else std::cout<<"null";
  }
  std::cout<<"]\n";
  hb_font_destroy(font);hb_face_destroy(face);hb_blob_destroy(blob);
}
