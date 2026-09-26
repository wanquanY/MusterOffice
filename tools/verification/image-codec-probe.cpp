// Isolated sanitizer test transport, little-endian host profile (macOS arm64).
#include "mo_decode.h"
#include "mo_skia.h"
#include <cstdio>
#include <vector>
int main() {
  uint32_t length;
  while (std::fread(&length, 4, 1, stdin) == 1) {
    if (length > 32u * 1024 * 1024) return 1;
    std::vector<uint8_t> encoded(length);
    if (std::fread(encoded.data(), 1, length, stdin) != length) return 1;
    uint8_t *pixels = nullptr;
    uint32_t info[9] = {};
    int32_t status = mo_image_decode(encoded.data(), length, &pixels, info);
    if (std::fwrite(&status, 4, 1, stdout) != 1 || std::fwrite(info, 4, 9, stdout) != 9)
      return 1;
    if (info[8] && std::fwrite(pixels, 1, info[8], stdout) != info[8]) return 1;
    mo_skia_free(pixels);
  }
}
