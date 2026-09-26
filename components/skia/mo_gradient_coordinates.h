#pragma once
#include <array>
#include <cstdint>

namespace mo::gradient {
struct Coordinates {
    std::array<float,6> matrix; // a,b,ox,c,d,oy; unit plane -> device pixels
    std::array<uint32_t,2> tile; // clamp, repeat, mirror
    bool centered; // elliptic output is (2*tiled(u)-1)*scale, rounded once
    std::array<float,2> scale;
};
struct MappedCoordinates {
    float x=0, y=0;
    bool valid=false, exact_fallback=false;
};
// Correct rounding for the supplied binary32 plane and device sample. This
// does not certify upstream source quantization, coverage or color interpolation.
class PreparedCoordinates {
public:
    explicit PreparedCoordinates(Coordinates);
    MappedCoordinates map(float x,float y) const;
private:
    Coordinates values_;
    double det_lo_=0, det_hi_=0;
    uint8_t axes_=0; // 1: diagonal, 2: exchanged axes
    bool valid_=false;
};
}
