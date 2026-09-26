#pragma once
#include <cstdint>

namespace mo::elliptic {
// Scalar geometry only. No Skia, allocation, I/O, font, color or host dependency.
// The outer ellipse is the unit circle. At t in [0,1], the inner ellipse's
// center and nonnegative radii interpolate linearly to (0,0) and (1,1).
struct Field { float cx, cy, sx, sy; };
enum class Status : uint32_t { Ok, Invalid, Precision, Limit };
enum class Location : uint32_t { Inner, Boundary, Outside, Unresolved };
struct Result {
    Status status = Status::Invalid;
    Location location = Location::Unresolved;
    float value = 0;
    double lower = 0, upper = 0;
    uint32_t nodes = 0, degree = 0;
    bool certified_fast_path = false;
};
// Includes roundoff of the returned float. Applies to this field's exact
// binary32 inputs only, not source geometry conversion or final pixel colors.
inline constexpr double max_scalar_error = 0x1p-23;
inline constexpr uint32_t max_nodes = 512;

// First closed-ellipse membership, including non-nested families and tangencies.
// Inner points return 0. No membership on [0,1] returns clamped 1. A successful
// boundary result encloses the first root and value in [lower,upper], of width
// <= max_scalar_error. Ambiguity is a diagnostic, never a guessed root/color.
// Input domain: |x|,|y| <= 1; |center|,radii <= 32768, radii >= 0, all finite.
// IEEE binary64 round-to-nearest, gradual underflow, no fast-math or contraction
// are required. Caller budget may reduce but never raise the fixed work cap.
Result evaluate(const Field&, float x, float y, uint32_t node_budget = max_nodes);
}
