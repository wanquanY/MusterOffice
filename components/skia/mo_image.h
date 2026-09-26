#pragma once
#include "include/core/SkImage.h"
#include "include/core/SkShader.h"
#include <cstdint>

// Packed, contiguous RGBA8 resources: [byte offset, width, height, alpha].
// alpha 0 = straight sRGB, 1 = premultiplied sRGB. No decoding or implicit ICC.
int mo_validate_images(const uint32_t* descriptors, uint32_t count,
                       const uint8_t* data, uint32_t bytes);
// [resource, tile-x, tile-y, sampling, m00, m01, m02, m10, m11, m12].
// Affine maps source pixel boundaries to world/device pixels. No mipmapping.
int mo_validate_image_brush(const uint32_t* brush, const uint32_t* descriptors,
                            uint32_t count);
sk_sp<SkImage> mo_make_image(const uint32_t* descriptor, const uint8_t* data);
sk_sp<SkShader> mo_make_image_brush(const uint32_t* brush, const sk_sp<SkImage>& image);
// V6 appends source pixel domain [left,top,right,bottom] to the V5 record.
int mo_validate_image_domain(const uint32_t* brush);
sk_sp<SkShader> mo_make_image_domain(const uint32_t* brush, const sk_sp<SkImage>& image);
