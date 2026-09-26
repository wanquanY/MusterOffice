#pragma once
#include "include/core/SkShader.h"
#include <cstdint>
#include <atomic>

// Legacy nine words; linear plane sixteen, rectangular plane seventeen.
// Plane geometry is basis[6], tileXY[2], then linear[3] or edgeRates[4].
inline uint32_t mo_gradient_header(uint32_t kind) {
    return kind == 4 ? 19 : kind == 3 ? 17 : kind == 2 ? 16 : 9;
}
// Per synchronous request; shared by all paints. Numerical failure never
// quarantines an otherwise healthy component or publishes partially drawn data.
struct MoGradientState {
    std::atomic<uint32_t> failure{0}, samples{0}, nodes{0}, coordinateFallbacks{0};
    static constexpr uint32_t maxSamples=16*1024*1024, maxNodes=64*1024*1024;
    static constexpr uint32_t maxCoordinateFallbacks=65536;
};
// Common fields: geometry, tile, interpolation, premul, stops, geometry[4].
// Followed by stops * [position, r, g, b, a] float32 bit patterns.
// Pure validation before any allocations. Advances pos only on success.
int mo_validate_gradient(const uint32_t* r, uint32_t words, uint32_t& pos,
                         uint32_t& total_stops, bool planes, bool office, bool rectangular, bool elliptic);
sk_sp<SkShader> mo_make_gradient(const uint32_t* r, MoGradientState* state);
sk_sp<SkShader> mo_make_gradient_plane(const uint32_t* p, sk_sp<SkShader> ramp, MoGradientState* state);
sk_sp<SkShader> mo_make_office_gradient(const uint32_t* p, sk_sp<SkShader> scalar);
