#pragma once
#include "include/core/SkData.h"
#include "include/core/SkImage.h"
#include "include/core/SkShader.h"
#include "include/core/SkSamplingOptions.h"
#include "include/core/SkTileMode.h"

// Copy exactly once per admitted prefix. The captured image owns its pixels;
// it cannot alias the mutable output surface or borrowed input resources.
inline sk_sp<SkShader> mo_snapshot_shader(const SkImageInfo& info, const uint8_t* pixels, size_t bytes) {
    auto data = SkData::MakeWithCopy(pixels, bytes);
    if (!data) return nullptr;
    auto image = SkImages::RasterFromData(info, std::move(data), size_t(info.width()) * 4);
    if (!image) return nullptr;
    return image->makeShader(SkTileMode::kClamp, SkTileMode::kClamp,
                             SkSamplingOptions(SkFilterMode::kNearest));
}
