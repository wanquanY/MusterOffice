// Development host for the native component ABI, never linked into the kernel.
#include "mo_hb.h"
#include "mo_hb_allocator.h"
#include <fstream>
#include <iostream>
#include <vector>
#include <cstdlib>
#include <limits>
static bool read(const char *path, size_t limit, std::vector<uint8_t> &out) {
  std::ifstream stream(path,std::ios::binary|std::ios::ate);
  if (!stream) return false;
  auto size=stream.tellg();
  if (size<0 || static_cast<uint64_t>(size)>limit) return false;
  out.resize(static_cast<size_t>(size));stream.seekg(0);
  stream.read(reinterpret_cast<char *>(out.data()),size);
  return bool(stream);
}
int main(int argc,char **argv) {
  if (argc<4 || argc>5) return 1;
  std::vector<uint8_t> font,input;
  if (!read(argv[1],128*1024*1024,font) || !read(argv[2],2*1024*1024,input) || input.size()%4) return 2;
  std::vector<uint32_t> request(input.size()/4);
  for (size_t i=0; i<request.size(); ++i)
    request[i]=uint32_t(input[i*4])|(uint32_t(input[i*4+1])<<8)|(uint32_t(input[i*4+2])<<16)|(uint32_t(input[i*4+3])<<24);
#ifdef MO_HB_FAULT_TEST
  if (argc==5) mo_hb_fail_after(std::strtoull(argv[4],nullptr,10));
#endif
  uint32_t *output=nullptr,length=0;
  auto call=[&] {
    if (std::string(argv[3])=="--outlines") return mo_hb_outline_font(font.data(),font.size(),request.data(),request.size(),&output,&length);
    if (std::string(argv[3])=="--carets") return mo_hb_caret_font(font.data(),font.size(),request.data(),request.size(),&output,&length);
    if (std::string(argv[3])=="--metrics") return mo_hb_measure_font(font.data(),font.size(),request.data(),request.size(),&output,&length);
    return mo_hb_shape(font.data(),font.size(),request.data(),request.size(),argv[3],std::char_traits<char>::length(argv[3]),&output,&length);
  };
  auto status=call();
  std::cout<<"{\"status\":"<<status<<",\"version\":"<<mo_hb_version()<<",\"words\":[";
  for (uint32_t i=0; i<length; ++i) { if (i) std::cout<<",";std::cout<<output[i]; }
  mo_hb_free(output);
  std::cout<<"],\"retainedBytes\":"<<mo_hb_alloc_live();
#ifdef MO_HB_FAULT_TEST
  if (argc==5) {
    mo_hb_fail_after(std::numeric_limits<size_t>::max());
    output=nullptr;length=0;
    status=call();
    std::cout<<",\"recoveryStatus\":"<<status<<",\"recoveryWords\":[";
    for (uint32_t i=0; i<length; ++i) { if (i) std::cout<<",";std::cout<<output[i]; }
    mo_hb_free(output);std::cout<<"]";
  }
#endif
  std::cout<<"}"<<std::endl;
  return 0;
}
