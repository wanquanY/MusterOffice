#include "mo_skia.h"
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

// One request per isolated process. The host controls stdin/stdout and timeout.
// Binary little-endian: word-count, request words; reply status, byte-length,
// then completed RGBA pixels. Diagnostics never share the binary stdout channel.
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
    const bool images_mode = argc == 2 && std::strcmp(argv[1], "--images") == 0;
    const bool pick_mode = argc == 2 && std::strcmp(argv[1], "--pick") == 0;
    if (argc != 1 && !images_mode && !pick_mode) return 2;
    uint32_t count;
    if (!read_word(count) || count > 2908303u) return 2;
    std::vector<uint32_t> request(count);
    for (auto& word : request) if (!read_word(word)) return 2;
    uint32_t input_bytes = 0;
    if (images_mode && (!read_word(input_bytes) || input_bytes > 67108864)) return 2;
    std::vector<uint8_t> data(input_bytes);
    if (input_bytes && std::fread(data.data(), 1, input_bytes, stdin) != input_bytes) return 2;
    if (std::getc(stdin) != EOF) return 2;
    if (pick_mode) {
        uint32_t* output = nullptr;
        uint32_t words = 0;
        const int status = mo_skia_pick(request.data(), count, &output, &words);
        bool success = write_word(uint32_t(status)) && write_word(words * 4);
        for (uint32_t i = 0; success && i < words; ++i) success = write_word(output[i]);
        mo_skia_free(output);
        return success ? 0 : 3;
    }
    uint8_t* pixels = nullptr;
    uint32_t bytes = 0;
    const int32_t status = images_mode
        ? mo_skia_raster_images(request.data(), count, data.data(), input_bytes, &pixels, &bytes)
        : mo_skia_raster(request.data(), count, &pixels, &bytes);
    const bool success = write_word(uint32_t(status)) && write_word(bytes) &&
                         (!bytes || std::fwrite(pixels, 1, bytes, stdout) == bytes);
    mo_skia_free(pixels);
    return success ? 0 : 3;
}
