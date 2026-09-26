#pragma once
#include <cstddef>
#include <cstdint>
namespace mo {
struct EncodedImage {
  uint32_t width = 0, height = 0, origin = 1, format = 0, depth = 0, color = 0;
};
// Complete bounded container validation, including CRC/ICC and declared frames.
int envelope(const uint8_t *bytes, size_t length, EncodedImage &image);
}
