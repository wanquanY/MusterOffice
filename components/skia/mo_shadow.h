#pragma once
#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstdlib>
#include <memory>

// Gaussian alpha convolution, evaluated in bounded continuation steps. The
// kernel is normalized and truncated at four sigma (tail mass < 0.000127 in
// two dimensions). Source alpha is sampled with transparent exterior pixels.
// Two reusable float planes are shared by all completed/nested groups.
class MoOuterShadow {
    std::unique_ptr<float, decltype(&std::free)> horizontal_{nullptr, &std::free};
    std::unique_ptr<float, decltype(&std::free)> mask_{nullptr, &std::free};
    std::array<double, 257> kernel_{};
    const uint8_t* source_ = nullptr;
    int width_=0,height_=0,radius_=0;
    size_t pixel_=0;
    bool vertical_=false;
    float dx_=0,dy_=0;
    uint32_t color_=0;
    float alpha(int x,int y) const {return x<0||y<0||x>=width_||y>=height_?0:mask_.get()[size_t(y)*width_+x];}
public:
    bool begin(const uint32_t* effect,int width,int height,const uint8_t* source) {
        width_=width;height_=height;source_=source;pixel_=0;vertical_=false;
        color_=effect[1];dx_=std::bit_cast<float>(effect[2]);dy_=std::bit_cast<float>(effect[3]);
        const double sigma=std::bit_cast<float>(effect[4]);
        radius_=int(std::ceil(sigma*4));
        const size_t count=size_t(width)*height;
        if(!horizontal_) horizontal_.reset(static_cast<float*>(std::malloc(count*sizeof(float))));
        if(!mask_) mask_.reset(static_cast<float*>(std::malloc(count*sizeof(float))));
        if(!horizontal_||!mask_)return false;
        double total=0;
        for(int k=-radius_;k<=radius_;++k) {double weight=radius_?std::exp(-double(k*k)/(2*sigma*sigma)):1;kernel_[k+radius_]=weight;total+=weight;}
        for(int k=0;k<=radius_*2;++k)kernel_[k]/=total;
        return true;
    }
    // True only after both passes have fully initialized their output plane.
    bool step() {
        const size_t count=size_t(width_)*height_;
        const size_t end=std::min(count,pixel_+std::max(size_t(1),size_t(4096)/(2*radius_+1)));
        for(;pixel_<end;++pixel_) {
            const int x=int(pixel_%width_),y=int(pixel_/width_);double value=0;
            for(int k=-radius_;k<=radius_;++k) {
                const int xx=vertical_?x:x+k, yy=vertical_?y+k:y;
                if(xx>=0&&yy>=0&&xx<width_&&yy<height_) {
                    const size_t index=size_t(yy)*width_+xx;
                    value+=kernel_[k+radius_]*(vertical_?horizontal_.get()[index]:source_[index*4+3]);
                }
            }
            (vertical_?mask_:horizontal_).get()[pixel_]=float(value);
        }
        if(pixel_<count)return false;
        if(!vertical_){vertical_=true;pixel_=0;return false;}
        return true;
    }
    uint32_t color() const {return color_;}
    uint32_t alpha(size_t pixel) const {
        const double x=double(pixel%width_)-dx_,y=double(pixel/width_)-dy_;
        const int ix=int(std::floor(x)),iy=int(std::floor(y));
        const double fx=x-ix,fy=y-iy;
        const double value=(alpha(ix,iy)*(1-fx)+alpha(ix+1,iy)*fx)*(1-fy)+(alpha(ix,iy+1)*(1-fx)+alpha(ix+1,iy+1)*fx)*fy;
        return uint32_t(std::clamp(std::floor(value*double(color_>>24)/255.0+0.5),0.0,255.0));
    }
};
