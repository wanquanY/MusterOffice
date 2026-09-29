#pragma once
#include "mo_pixel_work.h"
#include "include/core/SkSurface.h"
#include <array>
#include <cstdlib>
#include <memory>

// Same preorder interval grammar and explicit pixel budget as the Rust compiler.
// Complete validation precedes all group allocations. Prefix captures belong to
// the innermost group active BEFORE the draw at that prefix.
inline int mo_validate_opacity(const uint32_t* groups, uint32_t count,
    uint32_t draws, uint64_t pixels, uint32_t snapshots, uint16_t* scopes) {
    if (count > 4096) return 3;
    uint32_t stack[64]{}, depth = 0, maximum = 0, next = 0;
    for (uint32_t i = 0; i < draws; ++i) {
        while (depth && groups[stack[depth - 1] * 3 + 1] == i) --depth;
        while (next < count && groups[next * 3] <= i) {
            const auto* g = groups + next * 3;
            if (g[0] != i || g[1] <= i || g[1] > draws || g[2] > 65535 ||
                (depth && g[1] > groups[stack[depth - 1] * 3 + 1])) return 1;
            if (depth == 64) return 3;
            stack[depth++] = next++;
            maximum = std::max(maximum, depth);
        }
        scopes[i] = depth ? uint16_t(stack[depth - 1] + 1) : 0;
    }
    if (next != count) return 1;
    if (pixels * 4 * (maximum + snapshots) > 64 * 1024 * 1024 ||
        pixels * 2 * count > 268435456) return 3;
    return 0;
}

// No saveLayer/filter call hides unbounded work. Every allocation, clear, and
// merge is an explicit continuation step. Surfaces use viewport coordinates,
// preserving the existing path/AA scan rounding (including nested clips).
class MoOpacityStack {
    struct Layer {
        std::unique_ptr<uint8_t, decltype(&std::free)> pixels{nullptr, &std::free};
        sk_sp<SkSurface> surface; // destroyed before the pixels it wraps
        uint32_t id = 0;
    };
    const uint32_t* groups_;
    uint32_t count_, next_ = 0, depth_ = 0;
    std::array<std::unique_ptr<Layer>, 64> layers_;
    enum Phase { Idle, Clear, Merge } phase_ = Idle;
    size_t pixel_ = 0;
public:
    enum Step { Ready, Progress, AllocationFailure };
    MoOpacityStack(const uint32_t* groups, uint32_t count) : groups_(groups), count_(count) {}
    SkSurface* surface(SkSurface* root) const {
        return depth_ ? layers_[depth_ - 1]->surface.get() : root;
    }
    uint8_t* pixels(uint8_t* root) const {
        return depth_ ? layers_[depth_ - 1]->pixels.get() : root;
    }
    const uint8_t* snapshot_pixels(const uint8_t* root, uint32_t scope) const {
        if (!scope) return root;
        for (uint32_t i = 0; i < depth_; ++i)
            if (layers_[i]->id + 1 == scope) return layers_[i]->pixels.get();
        return nullptr;
    }
    bool boundary(uint32_t draw) const {
        return phase_ != Idle || (depth_ && groups_[layers_[depth_ - 1]->id * 3 + 1] == draw)
            || (next_ < count_ && groups_[next_ * 3] == draw);
    }
    Step advance(uint32_t draw, const SkImageInfo& info, uint8_t* root) {
        const size_t count = size_t(info.width()) * info.height();
        if (phase_ == Clear) {
            const size_t end = std::min(count, pixel_ + mo_pixels_per_work_unit);
            std::memset(layers_[depth_ - 1]->pixels.get() + pixel_ * 4, 0, (end - pixel_) * 4);
            pixel_ = end;
            if (end == count) {
                auto& layer = *layers_[depth_ - 1];
                if (!layer.surface)
                    layer.surface = SkSurfaces::WrapPixels(info, layer.pixels.get(), size_t(info.width()) * 4);
                if (!layer.surface) return AllocationFailure;
                phase_ = Idle;
            }
            return Progress;
        }
        if (phase_ == Merge) {
            const auto& layer = *layers_[depth_ - 1];
            const auto* src = layer.pixels.get();
            auto* dst = depth_ > 1 ? layers_[depth_ - 2]->pixels.get() : root;
            const uint32_t opacity = groups_[layer.id * 3 + 2];
            const size_t end = std::min(count, pixel_ + mo_pixels_per_work_unit);
            for (; pixel_ < end; ++pixel_) {
                const size_t i = pixel_ * 4;
                const uint32_t alpha = (uint32_t(src[i + 3]) * opacity + 32767) / 65535;
                for (size_t k = 0; k < 4; ++k) {
                    const uint32_t value = (uint32_t(src[i + k]) * opacity + 32767) / 65535;
                    dst[i + k] = uint8_t(value + (uint32_t(dst[i + k]) * (255 - alpha) + 127) / 255);
                }
            }
            if (end == count) {
                // All surfaces have the same viewport. One slot per depth is
                // reused by siblings; retained storage stays within maximum
                // depth, rather than growing with the number of objects.
                --depth_;
                phase_ = Idle;
            }
            return Progress;
        }
        if (depth_ && groups_[layers_[depth_ - 1]->id * 3 + 1] == draw) {
            phase_ = Merge; pixel_ = 0;
            return Progress;
        }
        if (next_ < count_ && groups_[next_ * 3] == draw) {
            auto& layer = layers_[depth_];
            if (!layer) {
                layer.reset(new (std::nothrow) Layer);
                if (!layer) return AllocationFailure;
                layer->pixels.reset(static_cast<uint8_t*>(std::malloc(count * 4)));
                if (!layer->pixels) return AllocationFailure;
            }
            layer->id = next_++;
            ++depth_;
            phase_ = Clear; pixel_ = 0;
            return Progress;
        }
        return Ready;
    }
};
