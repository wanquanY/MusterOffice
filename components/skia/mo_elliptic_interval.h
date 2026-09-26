#pragma once
#include <algorithm>
#include <cmath>
#include <limits>
#if defined(__FAST_MATH__) || (defined(__FINITE_MATH_ONLY__) && __FINITE_MATH_ONLY__)
#error "Elliptic interval arithmetic requires strict IEEE operations"
#endif

namespace mo::elliptic::detail {
static_assert(std::numeric_limits<float>::is_iec559 && std::numeric_limits<float>::digits==24);
static_assert(std::numeric_limits<double>::is_iec559 && std::numeric_limits<double>::digits==53);
// Outward binary64 intervals without changing process/thread rounding modes.
// Error-free sum/product recover the rounding direction. Exact zero arithmetic
// remains exact, which matters for point/line foci and polynomial endpoints.
struct Interval { double lo, hi; };
inline Interval exact(double v) { return {v,v}; }
inline Interval rounded(double value, double residual) {
    constexpr double inf = std::numeric_limits<double>::infinity();
    return {residual < 0 ? std::nextafter(value,-inf) : value,
            residual > 0 ? std::nextafter(value, inf) : value};
}
inline Interval sum(double a, double b) {
    const double value = a+b, bv = value-a;
    const double residual = (a-(value-bv))+(b-bv);
    return rounded(value,residual);
}
inline Interval product(double a, double b) {
    // Dekker splitting; the bounded field and depth keep all intermediates
    // normal or exact zero (including products of binary32 subnormals).
    constexpr double split = 0x1p27+1;
    const double value = a*b;
    const double ca = split*a, cb = split*b;
    const double ah = ca-(ca-a), bh = cb-(cb-b);
    const double al = a-ah, bl = b-bh;
    const double residual = ((ah*bh-value)+ah*bl+al*bh)+al*bl;
    return rounded(value,residual);
}
inline Interval add(Interval a, Interval b) {
    return {sum(a.lo,b.lo).lo,sum(a.hi,b.hi).hi};
}
inline Interval neg(Interval a) { return {-a.hi,-a.lo}; }
inline Interval sub(Interval a, Interval b) { return add(a,neg(b)); }
inline Interval mul(Interval a, Interval b) {
    const Interval v[] = {product(a.lo,b.lo),product(a.lo,b.hi),
                          product(a.hi,b.lo),product(a.hi,b.hi)};
    return {std::min({v[0].lo,v[1].lo,v[2].lo,v[3].lo}),
            std::max({v[0].hi,v[1].hi,v[2].hi,v[3].hi})};
}
inline Interval half(Interval a) { return {a.lo*0.5,a.hi*0.5}; }
inline Interval average(Interval a, Interval b) { return half(add(a,b)); }
inline Interval mix(Interval a, Interval b, double t) {
    return add(mul(a,sub(exact(1),exact(t))),mul(b,exact(t)));
}
}
