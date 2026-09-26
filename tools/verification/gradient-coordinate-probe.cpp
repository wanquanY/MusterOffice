#include "mo_gradient_coordinates.h"
#include <bit>
#include <cstdio>
#include <cstdint>
extern "C" void mo_coordinates_probe(const uint32_t* input,uint32_t* output) {
    for(int i=0;i<4;++i)output[i]=0;
    if(input[8]>1)return;
    mo::gradient::Coordinates v{};
    for(int i=0;i<6;++i)v.matrix[i]=std::bit_cast<float>(input[i]);
    v.tile={input[6],input[7]};v.centered=input[8];
    v.scale={std::bit_cast<float>(input[9]),std::bit_cast<float>(input[10])};
    const auto r=mo::gradient::PreparedCoordinates(v).map(std::bit_cast<float>(input[11]),std::bit_cast<float>(input[12]));
    output[0]=r.valid;output[1]=std::bit_cast<uint32_t>(r.x);output[2]=std::bit_cast<uint32_t>(r.y);output[3]=r.exact_fallback;
}
#ifndef __EMSCRIPTEN__
int main() {
    uint32_t n=0;if(std::fread(&n,4,1,stdin)!=1||n>1000000)return 1;
    for(uint32_t i=0;i<n;++i){uint32_t input[13],output[4];
        if(std::fread(input,4,13,stdin)!=13)return 2;
        mo_coordinates_probe(input,output);
        if(std::fwrite(output,4,4,stdout)!=4)return 3;}
    return std::fgetc(stdin)==EOF?0:4;
}
#endif
