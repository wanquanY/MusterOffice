#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct MoStretch MoStretch;
uint32_t mo_stretch_abi(void);
// Private, serial ABI. Pointers and lengths must describe valid non-overlapping
// storage. Status: 0 success; 1 input; 2 allocation; 3 poisoned; 4 DSP failure.
uint32_t mo_stretch_create(uint32_t channels, uint32_t block, uint32_t interval,
                           uint32_t max_bytes, MoStretch **out);
void mo_stretch_destroy(MoStretch *state);
uint32_t mo_stretch_latency(const MoStretch *state, uint32_t *input, uint32_t *output);
uint32_t mo_stretch_memory(const MoStretch *state, uint32_t *live, uint32_t *peak);
// Only legal once, immediately after create. Interleaved input length must be
// input_latency*channels. Finite playback rate is input/output, in [1/64,64].
uint32_t mo_stretch_begin(MoStretch *state, const float *input, uint32_t frames,
                         double playback_rate);
// Interleaved samples; max 65536 input frames and 4096 output frames per call.
// Caller owns exact rate/latency/tail/clock policy. Nonzero status invalidates
// output. Allocation/DSP failures poison state; input errors do not advance it.
uint32_t mo_stretch_process(MoStretch *state, const float *input, uint32_t input_frames,
                           float *output, uint32_t output_frames);
#ifdef MO_STRETCH_FAULT_TEST
void mo_stretch_fail_after(int64_t allocations);
#endif
#ifdef __cplusplus
}
#endif
