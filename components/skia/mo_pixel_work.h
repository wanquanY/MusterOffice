#pragma once
#include "include/core/SkData.h"
#include <algorithm>
#include <cstdint>
#include <cstring>

// Bounds explicit pixel preparation, not allocation latency, Skia scan conversion
// or a whole public step (which may request multiple work units).
constexpr uint32_t mo_pixels_per_work_unit = 65536;

// One private, unfinished buffer. The source remains immutable until take/drop.
// No partially normalized image or partially captured prefix can become a shader.
class MoPixelTransfer {
public:
    enum Mode { Copy, Premultiply };
    bool active() const { return bool(pixels_); }
    bool begin(const uint8_t* source, size_t bytes, Mode mode) {
        if (active() || !source || !bytes || bytes % 4) return false;
        pixels_ = SkData::MakeUninitialized(bytes);
        if (!pixels_) return false;
        source_ = source; bytes_ = bytes; offset_ = 0; mode_ = mode;
        return true;
    }
    bool advance() {
        if (!active()) return false;
        const size_t end = std::min(bytes_, offset_ + size_t(mo_pixels_per_work_unit) * 4);
        auto* out = static_cast<uint8_t*>(pixels_->writable_data());
        if (mode_ == Copy) {
            std::memcpy(out + offset_, source_ + offset_, end - offset_);
        } else {
            for (size_t i = offset_; i < end; i += 4) {
                for (size_t k = 0; k < 3; ++k)
                    out[i+k] = (uint32_t(source_[i+k]) * source_[i+3] + 127) / 255;
                out[i+3] = source_[i+3];
            }
        }
        offset_ = end;
        return offset_ == bytes_;
    }
    sk_sp<SkData> take() {
        if (!active() || offset_ != bytes_) return nullptr;
        source_ = nullptr; bytes_ = offset_ = 0;
        return std::move(pixels_);
    }
private:
    sk_sp<SkData> pixels_;
    const uint8_t* source_ = nullptr;
    size_t bytes_ = 0, offset_ = 0;
    Mode mode_ = Copy;
};
