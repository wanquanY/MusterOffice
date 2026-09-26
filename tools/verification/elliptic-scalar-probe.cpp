// Verification-only batch ABI. Production scalar implementation is linked,
// never duplicated. Six input binary32 words: x,y,cx,cy,sx,sy; then node budget.
#include "mo_elliptic_field.h"
#include "mo_elliptic_interval.h"
#include <bit>
#include <cstdint>
#include <cstdio>
#include <vector>
#include <cstring>
#ifndef __EMSCRIPTEN__
#include <chrono>
#endif

extern "C" void mo_elliptic_probe(const uint32_t* input, uint32_t* output, uint32_t count) {
    for (uint32_t i=0; i<count; ++i) {
        const auto* p=input+7*i;
        auto f=[](uint32_t v) { return std::bit_cast<float>(v); };
        const auto r=mo::elliptic::evaluate({f(p[2]),f(p[3]),f(p[4]),f(p[5])},f(p[0]),f(p[1]),p[6]);
        auto* q=output+10*i;
        q[0]=uint32_t(r.status); q[1]=uint32_t(r.location); q[2]=std::bit_cast<uint32_t>(r.value);
        const auto lo=std::bit_cast<uint64_t>(r.lower), hi=std::bit_cast<uint64_t>(r.upper);
        q[3]=uint32_t(lo); q[4]=uint32_t(lo>>32); q[5]=uint32_t(hi); q[6]=uint32_t(hi>>32);
        q[7]=r.nodes; q[8]=r.degree; q[9]=r.certified_fast_path;
    }
}
extern "C" void mo_elliptic_interval_probe(const uint32_t* input, uint32_t* output, uint32_t count) {
    for (uint32_t i=0; i<count; ++i) {
        double a,b,t;
        std::memcpy(&a,input+6*i,8); std::memcpy(&b,input+6*i+2,8); std::memcpy(&t,input+6*i+4,8);
        const auto s=mo::elliptic::detail::sum(a,b), p=mo::elliptic::detail::product(a,b);
        const auto m=mo::elliptic::detail::mix({a,a},{b,b},t);
        const double values[]={s.lo,s.hi,p.lo,p.hi,m.lo,m.hi};
        std::memcpy(output+12*i,values,48);
    }
}
#ifndef __EMSCRIPTEN__
int main(int argc, char** argv) {
    const bool interval=argc==2 && std::strcmp(argv[1],"interval")==0;
    const bool benchmark=argc==2 && std::strcmp(argv[1],"benchmark")==0;
    if (argc!=1 && !interval && !benchmark) return 1;
    uint32_t n;
    if (std::fread(&n,4,1,stdin)!=1 || n>1000000) return 1;
    std::vector<uint32_t> input(size_t(n)*(interval ? 6 : 7)), output(size_t(n)*(interval ? 12 : 10));
    if (std::fread(input.data(),4,input.size(),stdin)!=input.size() || std::fgetc(stdin)!=EOF) return 1;
    if (benchmark) {
        std::printf("{\"milliseconds\":[");
        for (unsigned trial=0; trial<8; ++trial) {
            const auto start=std::chrono::steady_clock::now();
            mo_elliptic_probe(input.data(),output.data(),n);
            const double ms=std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-start).count();
            if (trial) std::printf("%s%.9f",trial==1 ? "" : ",",ms);
        }
        uint32_t checksum=2166136261, statuses[4]{};
        for (auto word:output) checksum=(checksum^word)*16777619u;
        for (uint32_t i=0; i<n; ++i) { if (output[i*10]>3) return 3; ++statuses[output[i*10]]; }
        std::printf("],\"checksum\":%u,\"statuses\":[%u,%u,%u,%u]}\n",
            checksum,statuses[0],statuses[1],statuses[2],statuses[3]);
        return 0;
    }
    if (interval) mo_elliptic_interval_probe(input.data(),output.data(),n);
    else mo_elliptic_probe(input.data(),output.data(),n);
    if (std::fwrite(output.data(),4,output.size(),stdout)!=output.size()) return 2;
}
#endif
