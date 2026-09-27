#include "mo_image.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkData.h"
#include "include/core/SkMatrix.h"
#include "include/core/SkSamplingOptions.h"
#include "include/core/SkTileMode.h"
#include <algorithm>
#include <bit>
#include <cmath>

namespace {
float f(uint32_t bits) { return std::bit_cast<float>(bits); }
constexpr uint32_t max_bytes = 64 * 1024 * 1024;
}

int mo_validate_images(const uint32_t* d, uint32_t count,
                       const uint8_t* data, uint32_t bytes) {
    if (count > 4096 || bytes > max_bytes) return 3;
    if (bytes && !data) return 1;
    uint64_t end = 0;
    for (uint32_t i = 0; i < count; ++i) {
        const auto* p = d + 4*i;
        if (!p[1] || !p[2] || p[3] > 1 || p[0] != end) return 1;
        if (p[1] > 8192 || p[2] > 8192) return 3;
        end += uint64_t(p[1]) * p[2] * 4;
        if (end > max_bytes) return 3;
        if (end > bytes) return 1;
    }
    if (end != bytes) return 1;
    // Scan each byte at most once, after proving all descriptor spans valid.
    for (uint32_t i = 0; i < count; ++i) {
        const auto* p = d + 4*i;
        if (!p[3]) continue;
        const uint32_t size = p[1] * p[2] * 4;
        for (uint32_t j = 0; j < size; j += 4) {
            const auto* rgba = data + p[0] + j;
            if (rgba[0] > rgba[3] || rgba[1] > rgba[3] || rgba[2] > rgba[3]) return 1;
        }
    }
    return 0;
}

int mo_validate_image_brush(const uint32_t* p, const uint32_t* descriptors,
                            uint32_t count) {
    if (p[0] >= count || p[1] > 3 || p[2] > 3 || p[3] > 1) return 1;
    for (uint32_t i = 4; i < 10; ++i)
        if (!std::isfinite(f(p[i])) || std::abs(f(p[i])) > 32768) return 1;
    const double a = f(p[4]), b = f(p[5]), c = f(p[7]), d = f(p[8]);
    const double det = a*d - b*c, trace = a*a + b*b + c*c + d*d;
    if (trace == 0) return 3;
    const double largest = std::sqrt((trace + std::sqrt(std::max(0.0, trace*trace - 4*det*det))) / 2);
    if (std::abs(det) / largest < 1.0 / 16384.0) return 3;
    const auto* image = descriptors + p[0]*4;
    for (uint32_t x : {0u, image[1]}) for (uint32_t y : {0u, image[2]}) {
        if (std::abs((a*x + b*y) + f(p[6])) > 32768 ||
            std::abs((c*x + d*y) + f(p[9])) > 32768) return 3;
    }
    return 0;
}

sk_sp<SkImage> mo_make_image(const uint32_t* p, sk_sp<SkData> pixels) {
    if (!pixels || pixels->size() != size_t(p[1]) * p[2] * 4) return nullptr;
    const auto info = SkImageInfo::Make(p[1], p[2], kRGBA_8888_SkColorType,
        kPremul_SkAlphaType, SkColorSpace::MakeSRGB());
    return SkImages::RasterFromData(info, std::move(pixels), size_t(p[1]) * 4);
}

int mo_validate_image_domain(const uint32_t* p) {
    for (uint32_t i = 10; i < 14; ++i)
        if (!std::isfinite(f(p[i])) || std::abs(f(p[i])) > 32768) return 1;
    if (f(p[12]) - f(p[10]) < 1.0f/16384 || f(p[13]) - f(p[11]) < 1.0f/16384) return 3;
    for (float x : {f(p[10]), f(p[12])}) for (float y : {f(p[11]), f(p[13])}) {
        if (std::abs((double(f(p[4]))*x + double(f(p[5]))*y) + f(p[6])) > 32768 ||
            std::abs((double(f(p[7]))*x + double(f(p[8]))*y) + f(p[9])) > 32768) return 3;
    }
    return 0;
}

sk_sp<SkShader> mo_make_image_brush(const uint32_t* p, const sk_sp<SkImage>& image) {
    const SkTileMode tiles[] = {SkTileMode::kClamp, SkTileMode::kRepeat,
                              SkTileMode::kMirror, SkTileMode::kDecal};
    const auto matrix = SkMatrix::MakeAll(f(p[4]), f(p[5]), f(p[6]),
        f(p[7]), f(p[8]), f(p[9]), 0, 0, 1);
    return image->makeShader(tiles[p[1]], tiles[p[2]],
        SkSamplingOptions(p[3] ? SkFilterMode::kLinear : SkFilterMode::kNearest,
                          SkMipmapMode::kNone), matrix);
}
