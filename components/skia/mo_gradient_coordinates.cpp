#include "mo_gradient_coordinates.h"
#include "mo_gradient_coordinate_exact.h"
#include "mo_elliptic_interval.h"
#include <bit>
#include <limits>

namespace mo::gradient {
namespace {
using namespace mo::elliptic::detail;
constexpr double inf=std::numeric_limits<double>::infinity();
Interval divide(Interval n,Interval d) {
    if(d.hi<0){n=neg(n);d=neg(d);}
    if(d.lo<=0)return {-inf,inf};
    if(n.lo==0&&n.hi==0)return exact(0);
    if(n.lo==n.hi&&d.lo==d.hi){
        const double q=n.lo/d.lo;const auto back=product(q,d.lo);
        if(back.lo==n.lo&&back.hi==n.lo)return exact(q);
        return {std::nextafter(q,-inf),std::nextafter(q,inf)};
    }
    const double values[]={n.lo/d.lo,n.lo/d.hi,n.hi/d.lo,n.hi/d.hi};
    return {std::nextafter(*std::min_element(values,values+4),-inf),
            std::nextafter(*std::max_element(values,values+4),inf)};
}
bool tiled(Interval input,uint32_t mode,Interval& result) {
    if(mode==0){result={std::clamp(input.lo,0.0,1.0),std::clamp(input.hi,0.0,1.0)};return true;}
    const double period=mode==2?2:1;
    const double lo=std::floor(input.lo/period),hi=std::floor(input.hi/period);
    if(!std::isfinite(lo)||lo!=hi)return false;
    result=sub(input,exact(lo*period));
    if(mode==2){
        if(result.lo>=1)result=sub(exact(2),result);
        else if(result.hi>1)result={std::min(result.lo,2-result.hi),1};
    }
    return true;
}
bool rounded_float(Interval v,float& out) {
    if(!std::isfinite(v.lo)||!std::isfinite(v.hi))return false;
    const float lo=float(v.lo),hi=float(v.hi);
    if(lo!=hi)return false;
    out=lo==0?0:lo;return true;
}
Interval scale(Interval n,double s) {
    if(s==0)return exact(0);
    if(s==1)return n;
    if(s==2)return {n.lo*2,n.hi*2};
    if(n.lo==n.hi)return product(n.lo,s);
    if(s<0)return {product(n.hi,s).lo,product(n.lo,s).hi};
    return {product(n.lo,s).lo,product(n.hi,s).hi};
}
bool rounded_ratio(Interval n,double d,float& out) {
    if(n.lo==n.hi){
        const double q=n.lo/d;
        // For normal binary64 division, relative rounding error is <=2^-52.
        // A power-of-two padding of 2^-51 also covers rounding q +/- padding.
        // Equal binary32 endpoints certify the final rounding without calling
        // nextafter or expanding a point interval into four separate divides.
        const double padding=std::abs(q)*0x1p-51;
        if(padding>=std::numeric_limits<double>::min()&&std::isfinite(q)&&
           rounded_float({q-padding,q+padding},out))return true;
    }
    return rounded_float(divide(n,exact(d)),out);
}
// Axis-aligned and quarter-turned planes keep a single exact denominator.
// Fold the numerator before division and center before the final rounding.
// This is the same rational expression as the general inverse, with fewer
// interval operations, not a relaxed rounding tolerance.
bool mapped_ratio(Interval n,double d,uint32_t mode,bool centered,float factor,float& out) {
    if(d<0){d=-d;n=neg(n);}
    if(mode==0)n={std::clamp(n.lo,0.0,d),std::clamp(n.hi,0.0,d)};
    else if(!(n.lo>=0&&(mode==2?n.hi<=d:n.hi<d))) {
        const double period=mode==2?2*d:d;
        const auto q=divide(n,exact(period));
        const double lo=std::floor(q.lo),hi=std::floor(q.hi);
        if(!std::isfinite(lo)||lo!=hi)return false;
        n=sub(n,product(lo,period));
        if(mode==2){
            if(n.lo>=d)n=sub(exact(2*d),n);
            else if(n.hi>d)n={std::min(n.lo,2*d-n.hi),d};
        }
    }
    if(centered)n=scale(sub(scale(n,2),exact(d)),factor);
    return rounded_ratio(n,d,out);
}
}
PreparedCoordinates::PreparedCoordinates(Coordinates v):values_(v) {
    for(auto n:v.matrix)if(!std::isfinite(n)||std::abs(n)>32768)return;
    for(auto mode:v.tile)if(mode>2)return;
    for(auto scale:v.scale)if(!std::isfinite(scale)||scale<0||scale>1)return;
    const auto& m=v.matrix;
    const auto det=sub(product(m[0],m[4]),product(m[1],m[3]));
    // Uncertain determinants go through the exact integer path. An exactly
    // singular matrix is rejected there, rather than silently made invertible.
    if(det.lo<=0&&det.hi>=0){
        using detail::integer;
        const auto n=detail::sub(detail::mul(integer(m[0]),integer(m[4])),detail::mul(integer(m[1]),integer(m[3])));
        if(n.magnitude.zero()||n.magnitude.overflow)return;
    }
    det_lo_=det.lo;det_hi_=det.hi;valid_=true;
    if(m[1]==0&&m[3]==0)axes_=1;
    else if(m[0]==0&&m[4]==0)axes_=2;
}
MappedCoordinates PreparedCoordinates::map(float x,float y) const {
    MappedCoordinates result;
    if(!valid_||!std::isfinite(x)||!std::isfinite(y)||std::abs(x)>32768||std::abs(y)>32768)return result;
    const auto& m=values_.matrix;
    if(axes_){
        const bool swapped=axes_==2;
        if(mapped_ratio(sum(swapped?y:x,-double(m[swapped?5:2])),m[swapped?3:0],values_.tile[0],values_.centered,values_.scale[0],result.x)&&
           mapped_ratio(sum(swapped?x:y,-double(m[swapped?2:5])),m[swapped?1:4],values_.tile[1],values_.centered,values_.scale[1],result.y)){
            result.valid=true;return result;
        }
        result.exact_fallback=true;
        result.valid=detail::exact_map(values_,x,y,result.x,result.y);return result;
    }
    const auto px=sum(x,-double(m[2])),py=sum(y,-double(m[5]));
    const Interval nums[]={sub(scale(px,m[4]),scale(py,m[1])),
                           sub(scale(py,m[0]),scale(px,m[3]))};
    float* out[]={&result.x,&result.y};bool fast=true;
    for(uint32_t i=0;i<2;++i){
        if(det_lo_==det_hi_){
            if(!mapped_ratio(nums[i],det_lo_,values_.tile[i],values_.centered,values_.scale[i],*out[i])){fast=false;break;}
            continue;
        }
        Interval value;
        if(!tiled(divide(nums[i],{det_lo_,det_hi_}),values_.tile[i],value)){fast=false;break;}
        if(values_.centered)value=scale(sub(scale(value,2),exact(1)),values_.scale[i]);
        if(!rounded_float(value,*out[i])){fast=false;break;}
    }
    if(fast){result.valid=true;return result;}
    result.exact_fallback=true;
    result.valid=detail::exact_map(values_,x,y,result.x,result.y);
    return result;
}
}
