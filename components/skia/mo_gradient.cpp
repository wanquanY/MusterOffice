#include "mo_gradient.h"
#include "include/core/SkColorSpace.h"
#include "include/effects/SkGradient.h"
#include <algorithm>
#include <bit>
#include <cmath>
#include <vector>

namespace {
float f(uint32_t bits) { return std::bit_cast<float>(bits); }
bool coordinate(uint32_t bits) {
    return std::isfinite(f(bits)) && std::abs(f(bits)) <= 32768.0f;
}
}
int mo_validate_gradient(const uint32_t* r, uint32_t words, uint32_t& pos,
                         uint32_t& total_stops, bool planes, bool office, bool rectangular, bool elliptic) {
    if (pos > words || words - pos < 9) return 1;
    const auto* p = r + pos;
    if (p[0] > (elliptic ? 4u : rectangular ? 3u : planes ? 2u : 1u) || p[1] > 3 || p[2] > (office ? 2u : 1u) || p[3] > 1 || p[4] < 2) return 1;
    if (p[4] > 4096 || p[4] > 65536 - total_stops) return 3;
    const uint32_t header = mo_gradient_header(p[0]);
    if (words - pos < header || p[4] > (words - pos - header) / 5) return 1;
    if (p[0] >= 2) {
        for (uint32_t i = 5; i < header; ++i) {
            if (i == 11 || i == 12) { if (p[i] > 2) return 1; }
            else if (p[0] == 3 && i >= 13) {
                if (!std::isfinite(f(p[i])) || f(p[i]) < 0 || f(p[i]) >= 0x1p95f) return 1;
            } else if (!coordinate(p[i])) return 1;
        }
        if (p[0] == 4 && (f(p[13])<=0 || f(p[13])>1 || f(p[14])<=0 || f(p[14])>1 ||
                          f(p[17])<0 || f(p[18])<0)) return 1;
        const double a=f(p[5]), b=f(p[6]), c=f(p[8]), d=f(p[9]);
        const double det=a*d-b*c, trace=a*a+b*b+c*c+d*d;
        if (trace == 0) return 3;
        const double largest=std::sqrt((trace+std::sqrt(std::max(0.0,trace*trace-4*det*det)))/2);
        if (std::abs(det)/largest < 1.0/16384) return 3;
        for (double x : {0.0,1.0}) for (double y : {0.0,1.0}) {
            if (std::abs((a*x+b*y)+f(p[7])) > 32768 ||
                std::abs((c*x+d*y)+f(p[10])) > 32768) return 3;
        }
    } else {
    for (uint32_t i = 5; i < 9; ++i) if (!coordinate(p[i])) return 1;
    const double x = double(f(p[7])) - f(p[5]), y = double(f(p[8])) - f(p[6]);
    if (p[0] == 0 ? x*x + y*y < 1.0 / (16384.0 * 16384.0)
                  : f(p[7]) < 1.0f/16384.0f || p[8] != 0) return 3;
    }
    float last = 0;
    for (uint32_t i = 0; i < p[4]; ++i) {
        const auto* s = p + header + 5*i;
        const float position = f(s[0]);
        if (!std::isfinite(position) || position < last || position > 1) return 1;
        last = position;
        for (uint32_t j = 1; j < 5; ++j) {
            const float value = f(s[j]);
            if (!std::isfinite(value) || std::abs(value) > 65504 ||
                (j == 4 && (value < 0 || value > 1))) return 1;
        }
    }
    if (p[2] == 2) {
        if (p[3] != 0 || (p[4] != 2 && p[4] != 3)) return 1;
        const auto* first = p + header;
        const auto* last = first + 5*(p[4]-1);
        if (f(first[0]) != 0 || f(last[0]) != 1) return 1;
        if (p[4] == 3) {
            if (f(first[5]) <= 0 || f(first[5]) >= 1) return 1;
            for (uint32_t i=1; i<=4; ++i) if (f(first[i]) != f(last[i])) return 1;
        }
    }
    pos += header + 5*p[4];
    total_stops += p[4];
    return 0;
}
sk_sp<SkShader> mo_make_gradient(const uint32_t* p, MoGradientState* state) {
    const uint32_t header = mo_gradient_header(p[0]);
    std::vector<float> positions;
    std::vector<SkColor4f> colors;
    positions.reserve(p[4]); colors.reserve(p[4]);
    for (uint32_t i = 0; i < p[4]; ++i) {
        const auto* s = p + header + 5*i;
        positions.push_back(f(s[0]));
        colors.push_back({f(s[1]), f(s[2]), f(s[3]), f(s[4])});
    }
    const bool office = p[2] == 2;
    if (office) {
        // Geometry produces a unit scalar, with alpha solely the decal mask.
        positions = {0,1}; colors = {{0,0,0,1},{1,1,1,1}};
    }
    const SkTileMode tiles[] = {SkTileMode::kClamp, SkTileMode::kRepeat,
                              SkTileMode::kMirror, SkTileMode::kDecal};
    SkGradient::Interpolation interp;
    interp.fInPremul = p[3] ? SkGradient::Interpolation::InPremul::kYes
                            : SkGradient::Interpolation::InPremul::kNo;
    interp.fColorSpace = p[2] == 1 ? SkGradient::Interpolation::ColorSpace::kSRGBLinear
                              : SkGradient::Interpolation::ColorSpace::kSRGB;
    SkGradient gradient(SkGradient::Colors({colors.data(), colors.size()},
                                          {positions.data(), positions.size()}, tiles[p[1]],
                                          SkColorSpace::MakeSRGB()), interp);
    if (p[0] >= 2) {
        const SkPoint unit[] = {{0,0},{1,0}};
        auto shader = mo_make_gradient_plane(p, SkShaders::LinearGradient(unit, gradient), state);
        return office ? mo_make_office_gradient(p, std::move(shader)) : shader;
    }
    const SkPoint points[] = {{f(p[5]), f(p[6])}, {f(p[7]), f(p[8])}};
    auto shader = p[0] == 0 ? SkShaders::LinearGradient(points, gradient)
                             : SkShaders::RadialGradient(points[0], f(p[7]), gradient);
    return office ? mo_make_office_gradient(p, std::move(shader)) : shader;
}
