#include "mo_decode.h"
#include "mo_envelope.h"
#include "include/codec/SkCodec.h"
#include "include/codec/SkJpegDecoder.h"
#include "include/codec/SkPngDecoder.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkData.h"
#include "include/core/SkPixmap.h"
#include "include/core/SkSamplingOptions.h"
#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <memory>

extern "C" uint32_t mo_image_decode_abi() { return 1; }
extern "C" uint32_t mo_image_decode_sized_abi() { return 1; }
extern "C" int32_t mo_image_decode(const uint8_t *encoded, uint32_t length,
                                   uint8_t **pixels, uint32_t *info) {
  return mo_image_decode_sized(encoded, length, 0, 0, pixels, info);
}
extern "C" int32_t mo_image_decode_sized(const uint8_t *encoded, uint32_t length,
                                         uint32_t min_width, uint32_t min_height,
                                         uint8_t **pixels, uint32_t *info) {
  if (!pixels || !info) return 1;
  *pixels = nullptr;
  std::memset(info, 0, 9 * sizeof(uint32_t));
  if (!encoded || !length) return 1;
  if ((min_width == 0) != (min_height == 0) || min_width > 8192 || min_height > 8192) return 1;
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
  // Demand is in oriented axes. Original metadata stays intact for layout and
  // PPTX embedding; only this ephemeral preview sample grid is reduced.
  if (image.origin >= 5) std::swap(min_width, min_height);
  const double scale = min_width == 0 ? 1.0 : std::min(1.0, std::max(
      double(min_width) / image.width, double(min_height) / image.height));
  uint32_t sample_width = image.width, sample_height = image.height;
  if (image.format == 2 && scale < 1.0) {
    // JPEG decodes directly to a DCT grid at or above demand. Never allocate
    // the full RGBA plane merely to shrink it afterwards.
    auto dimensions = codec->getScaledDimensions(std::ceil(scale * 8.0) / 8.0);
    sample_width = dimensions.width(); sample_height = dimensions.height();
    if (sample_width < std::min(min_width, image.width) ||
        sample_height < std::min(min_height, image.height)) return 5;
  }
  size_t bytes = size_t(sample_width) * sample_height * 4;
  using Allocation = std::unique_ptr<uint8_t, decltype(&std::free)>;
  Allocation source(static_cast<uint8_t *>(std::malloc(bytes)), std::free);
  if (!source) return 2;
  const auto target = SkImageInfo::Make(sample_width, sample_height, kRGBA_8888_SkColorType,
                                       kPremul_SkAlphaType, SkColorSpace::MakeSRGB());
  // getPixels does not apply orientation. Do not accept partially filled images.
  if (codec->getPixels(target, source.get(), size_t(sample_width) * 4) != SkCodec::kSuccess)
    return 1;
  if (image.format == 1 && scale < 1.0) {
    // PNG has no DCT sampling. Bound scratch to one source plane, filter once,
    // and release it before the sampled image enters the retained page bundle.
    const uint32_t w = std::max(1u, uint32_t(std::ceil(image.width * scale)));
    const uint32_t h = std::max(1u, uint32_t(std::ceil(image.height * scale)));
    Allocation sampled(static_cast<uint8_t *>(std::malloc(size_t(w) * h * 4)), std::free);
    if (!sampled) return 2;
    SkPixmap from(target, source.get(), size_t(sample_width) * 4);
    SkPixmap to(target.makeWH(w, h), sampled.get(), size_t(w) * 4);
    if (!from.scalePixels(to, SkSamplingOptions(SkCubicResampler::Mitchell()))) return 5;
    source = std::move(sampled);
    sample_width = w; sample_height = h; bytes = size_t(w) * h * 4;
  }
  uint32_t width = sample_width, height = sample_height;
  if (image.origin >= 5) { width = sample_height; height = sample_width; }
  if (image.origin != 1) {
    Allocation oriented(static_cast<uint8_t *>(std::malloc(bytes)), std::free);
    if (!oriented) return 2;
    for (uint32_t y = 0; y < sample_height; ++y) {
      for (uint32_t x = 0; x < sample_width; ++x) {
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
                    source.get() + (size_t(y) * sample_width + x) * 4, 4);
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
