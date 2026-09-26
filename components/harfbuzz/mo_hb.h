#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// Borrowed aligned arrays for this call only. Output belongs to this component.
// Status: 0 success, 1 invalid input, 2 allocation/buffer failure, 3 shaper failure,
// 4 output budget, 5 unusable face, 6 instance invalidated.
// Allocation/buffer failure permanently invalidates this library instance. Recreate its
// process/WASM instance; never retry inside it. On failure output fields are zeroed.
int32_t mo_hb_shape(const uint8_t *font, uint32_t font_length,
                    const uint32_t *request, uint32_t request_words,
                    const char *language, uint32_t language_length,
                    uint32_t **output, uint32_t *output_words);
// Instance metrics use the same scale, ownership and permanent failure state.
int32_t mo_hb_measure_font(const uint8_t *font, uint32_t font_length,
                         const uint32_t *request, uint32_t request_words,
                         uint32_t **output, uint32_t *output_words);
// One font instance and many glyphs per call; monochrome outlines, unhinted.
int32_t mo_hb_outline_font(const uint8_t *font, uint32_t font_length,
                         const uint32_t *request, uint32_t request_words,
                         uint32_t **output, uint32_t *output_words);
void mo_hb_free(void *allocation);
uint32_t mo_hb_version(void);
#ifdef __cplusplus
}
#endif
