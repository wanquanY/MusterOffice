#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// Optional fixed Skia decode extension. No files, network, OS codecs or fonts.
// info[9]: oriented width,height, encoded width,height, EXIF orientation (1..8),
// format (1 PNG/2 JPEG), encoded bit depth, color (0 assumed sRGB/1 ICC/2 PNG),
// output byte length. Output is tightly packed premultiplied sRGB RGBA8.
// 0 success, 1 invalid, 2 allocation, 3 limit, 5 unsupported capability.
// All outputs remain zero on failure. Ownership uses mo_skia_free. Synchronous;
// host MUST isolate and terminate on trap/allocation failure/cancellation.
// Static images only: APNG and multi-picture JPEG fail explicitly.
uint32_t mo_image_decode_abi(void);
int32_t mo_image_decode(const uint8_t *encoded, uint32_t length,
                       uint8_t **pixels, uint32_t *info);
// Oriented minimum sample size; (0,0) means the exact original grid. Returned
// encoded dimensions/density always describe the original source. Native JPEG
// DCT grids can be larger than demand; a source is never enlarged.
uint32_t mo_image_decode_sized_abi(void);
int32_t mo_image_decode_sized(const uint8_t *encoded, uint32_t length,
                             uint32_t min_width, uint32_t min_height,
                             uint8_t **pixels, uint32_t *info);
#ifdef __cplusplus
}
#endif
