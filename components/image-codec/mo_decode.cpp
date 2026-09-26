#include "mo_decode.h"
#include "mo_envelope.h"
#include "include/codec/SkCodec.h"
#include "include/codec/SkJpegDecoder.h"
#include "include/codec/SkPngDecoder.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkData.h"
#include <cstdlib>
#include <cstring>
#include <memory>

extern "C" uint32_t mo_image_decode_abi() { return 1; }
extern "C" int32_t mo_image_decode(const uint8_t *encoded, uint32_t length,
                                   uint8_t **pixels, uint32_t *info) {
  if (!pixels || !info) return 1;
  *pixels = nullptr;
  std::memset(info, 0, 9 * sizeof(uint32_t));
  if (!encoded || !length) return 1;
  if (length > 32u * 1024 * 1024) return 3;
  mo::EncodedImage image;
  int status = mo::envelope(encoded, length, image);
  if (status) return status;
  auto data = SkData::MakeWithoutCopy(encoded, length);
  SkCodec::Result result;
  auto codec = image.format == 1 ? SkPngDecoder::Decode(data, &result)
                                 : SkJpegDecoder::Decode(data, &result);
  if (!codec || result != SkCodec::kSuccess) return 1;
  if (codec->dimensions().width() != static_cast<int>(image.width) ||
      codec->dimensions().height() != static_cast<int>(image.height) ||
      (image.color == 1 && !codec->getICCProfile())) return 1;
  const size_t bytes = size_t(image.width) * image.height * 4;
  using Allocation = std::unique_ptr<uint8_t, decltype(&std::free)>;
  Allocation source(static_cast<uint8_t *>(std::malloc(bytes)), std::free);
  if (!source) return 2;
  const auto target = SkImageInfo::Make(image.width, image.height, kRGBA_8888_SkColorType,
                                       kPremul_SkAlphaType, SkColorSpace::MakeSRGB());
  // getPixels does not apply orientation. Do not accept partially filled images.
  if (codec->getPixels(target, source.get(), size_t(image.width) * 4) != SkCodec::kSuccess)
    return 1;
  uint32_t width = image.width, height = image.height;
  if (image.origin >= 5) { width = image.height; height = image.width; }
  if (image.origin != 1) {
    Allocation oriented(static_cast<uint8_t *>(std::malloc(bytes)), std::free);
    if (!oriented) return 2;
    for (uint32_t y = 0; y < image.height; ++y) {
      for (uint32_t x = 0; x < image.width; ++x) {
        uint32_t dx = x, dy = y;
        switch (image.origin) {
          case 2: dx = width - 1 - x; break;
          case 3: dx = width - 1 - x; dy = height - 1 - y; break;
          case 4: dy = height - 1 - y; break;
          case 5: dx = y; dy = x; break;
          case 6: dx = width - 1 - y; dy = x; break;
          case 7: dx = width - 1 - y; dy = height - 1 - x; break;
          case 8: dx = y; dy = height - 1 - x; break;
        }
        std::memcpy(oriented.get() + (size_t(dy) * width + dx) * 4,
                    source.get() + (size_t(y) * image.width + x) * 4, 4);
      }
    }
    source = std::move(oriented);
  }
  const uint32_t metadata[9] = {width, height, image.width, image.height, image.origin,
                               image.format, image.depth, image.color, uint32_t(bytes)};
  std::memcpy(info, metadata, sizeof(metadata));
  *pixels = source.release();
  return 0;
}
