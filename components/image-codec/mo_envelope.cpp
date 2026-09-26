#include "mo_envelope.h"
#include "modules/skcms/skcms.h"
#include <zlib.h>
#include <cstdlib>
#include <cstring>
#include <memory>

namespace mo {
namespace {
constexpr size_t kMetadata = 4 * 1024 * 1024, kProfile = 1024 * 1024;
using Allocation = std::unique_ptr<uint8_t, decltype(&std::free)>;
uint32_t be32(const uint8_t *p) {
  return uint32_t(p[0]) << 24 | uint32_t(p[1]) << 16 | uint32_t(p[2]) << 8 | p[3];
}
uint32_t be16(const uint8_t *p) { return uint32_t(p[0]) << 8 | p[1]; }
int dimensions(const EncodedImage &image) {
  if (!image.width || !image.height) return 1;
  return image.width > 8192 || image.height > 8192 ||
         uint64_t(image.width) * image.height > 16 * 1024 * 1024 ? 3 : 0;
}
bool icc(const uint8_t *p, size_t n) {
  skcms_ICCProfile profile;
  return n >= 128 && n <= kProfile && be32(p) == n && skcms_Parse(p, n, &profile);
}
int exif(const uint8_t *p, size_t n, EncodedImage &image) {
  if (n < 8) return 1;
  bool little = p[0] == 'I' && p[1] == 'I';
  if (!little && !(p[0] == 'M' && p[1] == 'M')) return 1;
  auto u16 = [little](const uint8_t *v) -> uint32_t {
    return little ? uint32_t(v[1]) << 8 | v[0] : be16(v);
  };
  auto u32 = [little](const uint8_t *v) -> uint32_t {
    return little ? uint32_t(v[3]) << 24 | uint32_t(v[2]) << 16 |
                    uint32_t(v[1]) << 8 | v[0] : be32(v);
  };
  if (u16(p + 2) != 42) return 1;
  size_t offset = u32(p + 4);
  if (offset < 8 || offset > n - 2) return 1;
  size_t count = u16(p + offset);
  offset += 2;
  if (count > (n - offset) / 12 || n - offset - count * 12 < 4) return 1;
  bool seen = false;
  for (size_t i = 0; i < count; ++i) {
    const uint8_t *tag = p + offset + i * 12;
    if (u16(tag) != 0x112) continue;
    if (seen || u16(tag + 2) != 3 || u32(tag + 4) != 1) return 1;
    seen = true;
    image.origin = u16(tag + 8);
    if (image.origin < 1 || image.origin > 8) return 1;
  }
  return 0;
}
int png(const uint8_t *p, size_t n, EncodedImage &image) {
  image.format = 1;
  bool header = false, data = false, dataEnd = false, profile = false, orientation = false;
  size_t offset = 8, chunks = 0, metadata = 0;
  while (offset < n) {
    if (++chunks > 4096) return 3;
    if (n - offset < 12) return 1;
    size_t length = be32(p + offset);
    if (length > n - offset - 12) return 1;
    const uint8_t *type = p + offset + 4, *body = type + 4;
    if (crc32(0, type, uInt(length + 4)) != be32(body + length)) return 1;
    auto is = [type](const char *name) { return std::memcmp(type, name, 4) == 0; };
    if (!header && !is("IHDR")) return 1;
    if (!is("IDAT")) {
      metadata += length;
      if (metadata > kMetadata) return 3;
      if (data) dataEnd = true;
    }
    if (is("IHDR")) {
      if (header || length != 13) return 1;
      header = true;
      image.width = be32(body); image.height = be32(body + 4); image.depth = body[8];
      if (int status = dimensions(image)) return status;
    } else if (is("acTL") || is("fcTL") || is("fdAT") || is("cICP") ||
               is("cLLi") || is("mDCv")) {
      return 5; // Sequence and HDR pipelines need distinct capability contracts.
    } else if (is("iCCP")) {
      if (profile || data) return 1;
      profile = true;
      const auto *zero = static_cast<const uint8_t *>(std::memchr(body, 0, length));
      if (!zero || zero == body || zero - body > 79 || size_t(zero - body) + 2 >= length || zero[1])
        return 1;
      const size_t prefix = size_t(zero - body) + 2;
      Allocation decoded(static_cast<uint8_t *>(std::malloc(kProfile)), std::free);
      if (!decoded) return 2;
      uLongf decodedLength = kProfile;
      uLong encodedLength = length - prefix;
      const int z = uncompress2(decoded.get(), &decodedLength, body + prefix, &encodedLength);
      if (z == Z_BUF_ERROR) return 3;
      if (z == Z_MEM_ERROR) return 2;
      if (z != Z_OK || encodedLength != length - prefix || !icc(decoded.get(), decodedLength)) return 1;
      image.color = 1;
    } else if (is("eXIf")) {
      if (orientation) return 1;
      orientation = true;
      if (int status = exif(body, length, image)) return status;
    } else if (is("sRGB") || is("gAMA") || is("cHRM")) {
      if (data) return 1;
      if (is("sRGB") && (length != 1 || body[0] > 3)) return 1;
      if (is("gAMA") && (length != 4 || !be32(body))) return 1;
      if (is("cHRM")) {
        if (length != 32) return 1;
        float c[8];
        for (size_t i = 0; i < 8; ++i) c[i] = float(be32(body + 4 * i)) / 100000;
        skcms_Matrix3x3 matrix;
        if (!skcms_PrimariesToXYZD50(c[2], c[3], c[4], c[5], c[6], c[7], c[0], c[1], &matrix))
          return 1;
      }
      if (image.color != 1) image.color = 2;
    } else if (is("IDAT")) {
      if (dataEnd) return 1;
      data = true;
    } else if (is("IEND")) {
      return data && !length && offset + 12 == n ? 0 : 1;
    }
    offset += length + 12;
  }
  return 1;
}
int jpeg(const uint8_t *p, size_t n, EncodedImage &image) {
  image.format = 2;
  size_t offset = 2, markers = 0, metadata = 0, profileBytes = 0;
  bool entropy = false, scan = false, frame = false, orientation = false, end = false;
  const uint8_t *parts[255] = {};
  size_t lengths[255] = {};
  uint32_t partCount = 0;
  while (offset < n) {
    if (entropy) {
      while (offset < n && p[offset] != 0xff) ++offset;
      if (offset == n) return 1;
    }
    if (p[offset++] != 0xff) return 1;
    while (offset < n && p[offset] == 0xff) ++offset;
    if (offset == n) return 1;
    const uint8_t marker = p[offset++];
    if (entropy && (marker == 0 || (marker >= 0xd0 && marker <= 0xd7))) continue;
    entropy = false;
    if (++markers > 4096) return 3;
    if (marker == 0xd9) { end = offset == n; break; }
    if (!marker || marker == 0xd8 || marker == 1 || (marker >= 0xd0 && marker <= 0xd7)) return 1;
    if (n - offset < 2) return 1;
    size_t length = be16(p + offset);
    if (length < 2 || length > n - offset) return 1;
    const uint8_t *body = p + offset + 2;
    size_t bodyLength = length - 2;
    if (marker >= 0xe0 || marker == 0xfe) {
      metadata += bodyLength;
      if (metadata > kMetadata) return 3;
    }
    if (marker >= 0xc0 && marker <= 0xcf && marker != 0xc4 && marker != 0xc8 && marker != 0xcc) {
      if (frame || bodyLength < 6) return 1;
      if (marker != 0xc0 && marker != 0xc1 && marker != 0xc2) return 5;
      frame = true;
      image.depth = body[0]; image.height = be16(body + 1); image.width = be16(body + 3);
      if (image.depth != 8) return 5;
      if (int status = dimensions(image)) return status;
    } else if (marker == 0xe1 && bodyLength >= 6 && std::memcmp(body, "Exif\0\0", 6) == 0) {
      if (orientation) return 1;
      orientation = true;
      if (int status = exif(body + 6, bodyLength - 6, image)) return status;
    } else if (marker == 0xe2 && bodyLength >= 12 && std::memcmp(body, "ICC_PROFILE\0", 12) == 0) {
      if (bodyLength < 14 || !body[12] || !body[13] || body[12] > body[13]) return 1;
      if (partCount && partCount != body[13]) return 1;
      partCount = body[13];
      size_t i = body[12] - 1;
      if (parts[i]) return 1;
      parts[i] = body + 14; lengths[i] = bodyLength - 14; profileBytes += lengths[i];
      if (profileBytes > kProfile) return 3;
    } else if (marker == 0xe2 && bodyLength >= 4 && std::memcmp(body, "MPF\0", 4) == 0) {
      return 5;
    } else if (marker == 0xda) {
      if (!frame) return 1;
      scan = entropy = true;
    }
    offset += length;
  }
  if (!end || !frame || !scan) return 1;
  if (partCount) {
    if (!profileBytes) return 1;
    Allocation profile(static_cast<uint8_t *>(std::malloc(profileBytes)), std::free);
    if (!profile) return 2;
    size_t cursor = 0;
    for (size_t i = 0; i < partCount; ++i) {
      if (!parts[i]) return 1;
      std::memcpy(profile.get() + cursor, parts[i], lengths[i]); cursor += lengths[i];
    }
    if (!icc(profile.get(), profileBytes)) return 1;
    image.color = 1;
  }
  return 0;
}
}
int envelope(const uint8_t *p, size_t n, EncodedImage &image) {
  if (n >= 8 && std::memcmp(p, "\x89PNG\r\n\x1a\n", 8) == 0) return png(p, n, image);
  if (n >= 2 && p[0] == 0xff && p[1] == 0xd8) return jpeg(p, n, image);
  return 5;
}
}
