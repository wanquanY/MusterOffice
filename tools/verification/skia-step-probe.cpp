#include "mo_skia.h"
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <vector>

// Explicit test process: flags are images(0/1), work units, optional cancel
// after N steps. Input/output bytes match skia-probe; metrics use stderr only.
static bool read_word(uint32_t& value) {
    uint8_t b[4];
    if (std::fread(b, 1, 4, stdin) != 4) return false;
    value = uint32_t(b[0]) | uint32_t(b[1]) << 8 | uint32_t(b[2]) << 16 | uint32_t(b[3]) << 24;
    return true;
}
static bool write_word(uint32_t value) {
    const uint8_t b[] = {uint8_t(value), uint8_t(value >> 8), uint8_t(value >> 16), uint8_t(value >> 24)};
    return std::fwrite(b, 1, 4, stdout) == 4;
}
int main(int argc, char** argv) {
    if (argc != 3 && argc != 4) return 10;
    const auto images = uint32_t(std::strtoul(argv[1], nullptr, 10));
    const auto budget = uint32_t(std::strtoul(argv[2], nullptr, 10));
    const auto cancel_at = argc == 4 ? uint32_t(std::strtoul(argv[3], nullptr, 10)) : 0;
    if (images > 1 || !budget || budget > 4096 || mo_skia_execution_abi() != 1) return 11;
    uint32_t count;
    if (!read_word(count) || count > 2908303) return 12;
    std::vector<uint32_t> frame(count);
    for (auto& word : frame) if (!read_word(word)) return 13;
    uint32_t image_bytes = 0;
    if (images && (!read_word(image_bytes) || image_bytes > 67108864)) return 14;
    std::vector<uint8_t> data(image_bytes);
    if (image_bytes && std::fread(data.data(), 1, image_bytes, stdin) != image_bytes) return 15;
    if (std::getc(stdin) != EOF) return 16;
    MoSkiaRasterTask* raw = nullptr;
    int status = mo_skia_raster_begin(frame.data(), count, images ? data.data() : nullptr, image_bytes, images, &raw);
    std::unique_ptr<MoSkiaRasterTask, decltype(&mo_skia_raster_drop)> task(raw, mo_skia_raster_drop);
    if ((status == 0) != bool(raw)) return 17;
    uint32_t complete = 0, steps = 0, cancelled_steps = 0;
    double maximum_ms = 0;
    uint8_t* pixels = nullptr;
    uint32_t length = 0;
    if (!status) {
        if (mo_skia_raster_take(raw, &pixels, &length) != 1 || pixels || length) return 18;
        if (mo_skia_raster_step(raw, 0, &complete) != 1 || complete) return 19;
    }
    while (!status && !complete) {
        const auto start = std::chrono::steady_clock::now();
        status = mo_skia_raster_step(raw, budget, &complete);
        maximum_ms = std::max(maximum_ms, std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now()-start).count());
        ++steps;
        if (cancel_at == steps && !cancelled_steps && !complete && !status) {
            task.reset(); cancelled_steps = steps;
            status = mo_skia_raster_begin(frame.data(), count, images ? data.data() : nullptr, image_bytes, images, &raw);
            task.reset(raw);
        }
        if (!complete && !status && (mo_skia_raster_take(raw, &pixels, &length) != 1 || pixels || length)) return 20;
    }
    if (!status) {
        if (mo_skia_raster_step(raw, budget, &complete) || !complete) return 21;
        status = mo_skia_raster_take(raw, &pixels, &length);
        uint8_t* again = reinterpret_cast<uint8_t*>(1);
        uint32_t bytes = 1;
        if (mo_skia_raster_take(raw, &again, &bytes) != 1 || again || bytes) return 22;
        if (mo_skia_raster_step(raw, budget, &complete) != 1 || complete) return 23;
    }
    const bool success = write_word(uint32_t(status)) && write_word(length) &&
                         (!length || std::fwrite(pixels, 1, length, stdout) == length);
    mo_skia_free(pixels);
    std::fprintf(stderr, "{\"steps\":%u,\"cancelledSteps\":%u,\"maxStepMs\":%.6f}\n", steps, cancelled_steps, maximum_ms);
    return success ? 0 : 24;
}
