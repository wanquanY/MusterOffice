#pragma once
#include "include/core/SkData.h"
#include "include/core/SkImage.h"
#include "include/core/SkShader.h"
#include "include/core/SkSamplingOptions.h"
#include "include/core/SkTileMode.h"

// Only a complete private prefix capture may become a shader. Its data cannot
// alias the mutable output surface or borrowed input resources.
inline sk_sp<SkShader> mo_snapshot_shader(const SkImageInfo& info, sk_sp<SkData> data) {
    if (!data || data->size() != size_t(info.width()) * info.height() * 4) return nullptr;
    auto image = SkImages::RasterFromData(info, std::move(data), size_t(info.width()) * 4);
    if (!image) return nullptr;
    return image->makeShader(SkTileMode::kClamp, SkTileMode::kClamp,
                             SkSamplingOptions(SkFilterMode::kNearest));
}
