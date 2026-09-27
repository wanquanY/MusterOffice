#include "mo_pixel_work.h"
#include <cstdio>
#include <vector>

// Independent boundary check for the retained pixel buffer. Not a renderer
// performance benchmark; actual cross-component cancellation is tested separately.
int main() {
    uint32_t cases = 0, cancellations = 0;
    for (const size_t count : {size_t(1), size_t(65535), size_t(65536), size_t(65537), size_t(262145)}) {
        std::vector<uint8_t> src(count * 4);
        for (size_t i = 0; i < count; ++i) {
            src[4*i] = i % 256; src[4*i+1] = (i*37) % 256;
            src[4*i+2] = 255-i % 256; src[4*i+3] = (i/256) % 256;
        }
        for (const auto mode : {MoPixelTransfer::Copy, MoPixelTransfer::Premultiply}) {
            const size_t expected_steps = (count + 65535) / 65536;
            for (size_t stop = 0; stop < expected_steps; ++stop) {
                MoPixelTransfer partial;
                if (!partial.begin(src.data(), src.size(), mode) || partial.take()) return 1;
                for (size_t i = 0; i < stop; ++i) if (partial.advance() || partial.take()) return 2;
                ++cancellations; // Destructor must release the unfinished private allocation.
            }
            MoPixelTransfer task;
            if (task.active() || task.advance() || task.take()) return 3;
            if (task.begin(nullptr, src.size(), mode) || task.begin(src.data(), 0, mode) ||
                task.begin(src.data(), 3, mode)) return 4;
            for (int reuse = 0; reuse < 2; ++reuse) {
                if (!task.begin(src.data(), src.size(), mode) || !task.active() || task.take()) return 5;
                if (task.begin(src.data(), src.size(), mode)) return 6;
                size_t steps = 0;
                while (!task.advance()) {++steps; if (task.take() || steps >= expected_steps) return 7;}
                if (++steps != expected_steps || !task.advance()) return 8;
                const auto pixels = task.take();
                if (!pixels || pixels->size() != src.size() || task.active() || task.take()) return 9;
                const auto* out = static_cast<const uint8_t*>(pixels->data());
                for (size_t i = 0; i < count; ++i) for (size_t c = 0; c < 4; ++c) {
                    // Round a nonnegative rational using quotient/remainder,
                    // independent of the implementation's biased division.
                    unsigned expected = src[4*i+c];
                    if (mode == MoPixelTransfer::Premultiply && c < 3) {
                        const unsigned product = expected * src[4*i+3];
                        expected = product/255 + unsigned(2*(product%255) >= 255);
                    }
                    if (out[4*i+c] != expected) return 10;
                }
                ++cases;
            }
        }
    }
    std::printf("{\"status\":\"passed\",\"completed\":%u,\"cancelled\":%u}\n", cases, cancellations);
}
