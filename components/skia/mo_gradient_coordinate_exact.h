#pragma once
#include "mo_gradient_coordinates.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cmath>

namespace mo::gradient::detail {
// Every finite input is <=32768, represented as an integer times 2^-149.
// Matrix products need <=332 bits; scaled centered quotients and comparisons
// to float midpoints need <=636 bits. 768 bits leave explicit checked headroom.
struct UInt {
    static constexpr uint32_t words=24;
    std::array<uint32_t,words> v{};
    bool overflow=false;
    bool zero() const { for (auto x:v) if (x) return false; return true; }
    int compare(const UInt& b) const {
        for (uint32_t i=words;i--;) if (v[i]!=b.v[i]) return v[i]<b.v[i] ? -1 : 1;
        return 0;
    }
    uint32_t bits() const {
        for (uint32_t i=words;i--;) if (v[i]) return i*32+32-std::countl_zero(v[i]);
        return 0;
    }
    UInt shift(uint32_t n) const {
        UInt r; r.overflow=overflow;
        for (uint32_t i=0;i<words;++i) if (v[i]) {
            const uint64_t x=uint64_t(v[i])<<(n%32);const uint64_t j=uint64_t(i)+n/32;
            if (j<words) r.v[j]|=uint32_t(x); else r.overflow=true;
            if (x>>32) { if(j+1<words) r.v[j+1]|=uint32_t(x>>32); else r.overflow=true; }
        }
        return r;
    }
};
inline UInt add(const UInt& a,const UInt& b) {
    UInt r;uint64_t carry=0;r.overflow=a.overflow||b.overflow;
    for(uint32_t i=0;i<UInt::words;++i){const uint64_t s=uint64_t(a.v[i])+b.v[i]+carry;r.v[i]=uint32_t(s);carry=s>>32;}
    r.overflow|=carry!=0;return r;
}
inline UInt subtract(const UInt& a,const UInt& b) {
    UInt r;uint64_t borrow=0;r.overflow=a.overflow||b.overflow;
    for(uint32_t i=0;i<UInt::words;++i){const uint64_t s=uint64_t(b.v[i])+borrow;r.v[i]=uint32_t(uint64_t(a.v[i])-s);borrow=uint64_t(a.v[i])<s;}
    r.overflow|=borrow!=0;return r;
}
inline UInt multiply(const UInt& a,const UInt& b) {
    UInt r;r.overflow=a.overflow||b.overflow;
    for(uint32_t i=0;i<UInt::words;++i) if(a.v[i]) {
        uint64_t carry=0;
        for(uint32_t j=0;j<UInt::words;++j) {
            if(i+j>=UInt::words){r.overflow|=b.v[j]!=0||carry!=0;carry=0;continue;}
            const uint64_t s=uint64_t(a.v[i])*b.v[j]+r.v[i+j]+carry;
            r.v[i+j]=uint32_t(s);carry=s>>32;
        }
        r.overflow|=carry!=0;
    }
    return r;
}
struct Int { UInt magnitude; bool negative=false; };
inline Int clean(Int v) { if(v.magnitude.zero())v.negative=false;return v; }
inline Int negate(Int v) {v.negative=!v.negative;return clean(v);}
inline Int integer(float f) {
    const uint32_t bits=std::bit_cast<uint32_t>(f),exponent=(bits>>23)&255;
    UInt n;n.v[0]=(bits&0x7fffff)|(exponent?0x800000:0);
    if(exponent)n=n.shift(exponent-1);
    return clean({n,(bits>>31)!=0});
}
inline Int add(const Int& a,const Int& b) {
    if(a.negative==b.negative)return clean({add(a.magnitude,b.magnitude),a.negative});
    if(a.magnitude.compare(b.magnitude)>=0)return clean({subtract(a.magnitude,b.magnitude),a.negative});
    return clean({subtract(b.magnitude,a.magnitude),b.negative});
}
inline Int sub(const Int& a,const Int& b){return add(a,negate(b));}
inline Int mul(const Int& a,const Int& b){return clean({multiply(a.magnitude,b.magnitude),a.negative!=b.negative});}
inline UInt remainder(UInt n,const UInt& d) {
    if(d.zero()){n.overflow=true;return n;}
    while(n.compare(d)>=0&&!n.overflow){uint32_t shift=n.bits()-d.bits();UInt value=d.shift(shift);
        if(value.compare(n)>0){--shift;value=d.shift(shift);}n=subtract(n,value);}
    return n;
}
inline UInt tile(Int n,const UInt& denominator,uint32_t mode) {
    if(mode==0){if(n.negative)return {};return n.magnitude.compare(denominator)>0?denominator:n.magnitude;}
    const UInt period=mode==2?denominator.shift(1):denominator;
    UInt result=remainder(n.magnitude,period);
    if(n.negative&&!result.zero())result=subtract(period,result);
    if(mode==2&&result.compare(denominator)>0)result=subtract(period,result);
    return result;
}
// Ratio magnitude is in [0,1]. Find the adjacent binary32 values by exact
// comparison, then select at their exact midpoint with ties to even.
inline bool round(Int numerator,UInt denominator,float& result) {
    if(numerator.magnitude.overflow||denominator.overflow||denominator.zero()||numerator.magnitude.compare(denominator)>0)return false;
    const UInt left=numerator.magnitude.shift(149);
    uint32_t lo=0,hi=0x3f800000;
    while(lo<hi){const uint32_t mid=lo+(hi-lo+1)/2;const UInt value=multiply(denominator,integer(std::bit_cast<float>(mid)).magnitude);
        if(value.overflow||left.overflow)return false;
        if(left.compare(value)>=0)lo=mid;else hi=mid-1;}
    if(lo!=0x3f800000){
        const UInt sum=add(integer(std::bit_cast<float>(lo)).magnitude,integer(std::bit_cast<float>(lo+1)).magnitude);
        const UInt right=multiply(denominator,sum),twice=left.shift(1);
        if(right.overflow||twice.overflow)return false;
        const int c=twice.compare(right);if(c>0||(c==0&&(lo&1)))++lo;
    }
    if(lo&&numerator.negative)lo|=0x80000000;
    result=std::bit_cast<float>(lo);return true;
}
inline bool exact_map(const Coordinates& v,float x,float y,float& out_x,float& out_y) {
    const auto& m=v.matrix;
    const Int a=integer(m[0]),b=integer(m[1]),c=integer(m[3]),d=integer(m[4]);
    const Int px=sub(integer(x),integer(m[2])),py=sub(integer(y),integer(m[5]));
    Int det=sub(mul(a,d),mul(b,c));
    if(det.magnitude.zero()||det.magnitude.overflow)return false;
    Int nums[]={sub(mul(px,d),mul(py,b)),sub(mul(py,a),mul(px,c))};
    float* outputs[]={&out_x,&out_y};
    for(uint32_t i=0;i<2;++i){
        if(det.negative)nums[i]=negate(nums[i]);
        if(nums[i].magnitude.overflow)return false;
        UInt n=tile(nums[i],det.magnitude,v.tile[i]),denominator=det.magnitude;
        Int numerator{n,false};
        if(v.centered){numerator=sub(Int{n.shift(1),false},Int{denominator,false});
            numerator=mul(numerator,integer(v.scale[i]));denominator=denominator.shift(149);}
        if(!round(numerator,denominator,*outputs[i]))return false;
    }
    return true;
}
}
