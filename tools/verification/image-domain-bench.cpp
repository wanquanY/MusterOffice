#include "mo_skia.h"
#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <vector>

// Diagnostic host only. Input is the same bounded frame/bundle as skia-probe.
bool word(uint32_t& out) {
    uint8_t b[4];
    if (std::fread(b,1,4,stdin)!=4) return false;
    out=uint32_t(b[0]) | uint32_t(b[1])<<8 | uint32_t(b[2])<<16 | uint32_t(b[3])<<24;
    return true;
}
int main() {
    uint32_t count,size;
    if (!word(count) || count>2691084) return 2;
    std::vector<uint32_t> frame(count);
    for (auto& v:frame) if(!word(v)) return 2;
    if(!word(size) || size>67108864) return 2;
    std::vector<uint8_t> source(size);
    if(std::fread(source.data(),1,size,stdin)!=size || std::getc(stdin)!=EOF) return 2;
    std::vector<uint8_t> baseline;
    std::vector<double> times;
    for (int i=0;i<28;++i) {
        uint8_t* pixels=nullptr;uint32_t length=0;
        const auto start=std::chrono::steady_clock::now();
        const int status=mo_skia_raster_images(frame.data(),count,source.data(),size,&pixels,&length);
        const double elapsed=std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-start).count();
        if(status || !pixels) return 3;
        if(baseline.empty()) baseline.assign(pixels,pixels+length);
        else if(length!=baseline.size() || !std::equal(baseline.begin(),baseline.end(),pixels)) return 4;
        mo_skia_free(pixels);
        if(i>=3)times.push_back(elapsed);
    }
    std::printf("{\"samplesMs\":[");
    for(size_t i=0;i<times.size();++i)std::printf("%s%.9f",i?",":"",times[i]);
    std::printf("],\"pixelBytes\":%zu}\n",baseline.size());
    return std::fwrite(baseline.data(),1,baseline.size(),stdout)==baseline.size() ? 0 : 5;
}
