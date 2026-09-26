#include "mo_elliptic_prepared.h"
#include <bit>
#include <cstdint>
#include <cstdio>
#include <vector>
extern "C" void mo_elliptic_prepared_probe(const uint32_t* input,uint32_t* output,uint32_t count) {
    for(uint32_t i=0;i<count;++i) {
        const auto* p=input+7*i;auto f=[](uint32_t v){return std::bit_cast<float>(v);};
        mo::elliptic::PreparedField field({f(p[2]),f(p[3]),f(p[4]),f(p[5])});
        const auto r=field.evaluate(f(p[0]),f(p[1]),p[6]);auto* q=output+10*i;
        q[0]=uint32_t(r.status);q[1]=uint32_t(r.location);q[2]=std::bit_cast<uint32_t>(r.value);
        const auto lo=std::bit_cast<uint64_t>(r.lower),hi=std::bit_cast<uint64_t>(r.upper);
        q[3]=uint32_t(lo);q[4]=uint32_t(lo>>32);q[5]=uint32_t(hi);q[6]=uint32_t(hi>>32);
        q[7]=r.nodes;q[8]=r.degree;q[9]=r.certified_fast_path;
    }
}
#ifndef __EMSCRIPTEN__
int main() {
    uint32_t n;if(std::fread(&n,4,1,stdin)!=1 || n>1000000)return 1;
    std::vector<uint32_t> in(size_t(n)*7),out(size_t(n)*10);
    if(std::fread(in.data(),4,in.size(),stdin)!=in.size() || std::fgetc(stdin)!=EOF)return 1;
    mo_elliptic_prepared_probe(in.data(),out.data(),n);
    return std::fwrite(out.data(),4,out.size(),stdout)==out.size()?0:2;
}
#endif
