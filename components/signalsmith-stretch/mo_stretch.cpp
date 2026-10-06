#include "mo_stretch.h"
#include "mo_stretch_memory.h"
#include "signalsmith-stretch.h"
#include <cmath>
#include <memory>

using Engine = signalsmith::stretch::SignalsmithStretch<float, std::minstd_rand>;
extern "C" uint32_t mo_stretch_abi(void) { return 1; }
struct MoStretch {
  mo_stretch::Budget budget;
  // DSP lifetime is nested inside the allocator budget lifetime.
  std::unique_ptr<Engine> engine;
  uint32_t channels;
  bool started = false, poisoned = false;
  MoStretch(uint32_t channels_, uint32_t bytes)
      : budget{sizeof(MoStretch) + sizeof(Engine), sizeof(MoStretch) + sizeof(Engine), bytes}, channels(channels_) {}
};
namespace {
template<class T> struct Interleaved {
  T *samples;
  uint32_t channels;
  struct Channel {
    T *samples;
    uint32_t stride;
    T &operator[](int index) { return samples[index*stride]; }
  };
  Channel operator[](int channel) { return {samples + channel, channels}; }
};
bool finite(const float *samples, uint32_t frames, uint32_t channels) {
  if (frames && !samples) return false;
  for (size_t i = 0; i < size_t(frames)*channels; ++i)
    if (!std::isfinite(samples[i])) return false;
  return true;
}
}
extern "C" uint32_t mo_stretch_create(uint32_t channels, uint32_t block, uint32_t interval,
                                      uint32_t max_bytes, MoStretch **out) {
  if (!out) return 1;
  *out = nullptr;
  if (!channels || channels > 64 || block < 128 || block > 65536 ||
      !interval || interval > block/2 || max_bytes > 128*1024*1024 ||
      max_bytes < sizeof(MoStretch) + sizeof(Engine)) return 1;
  try {
    auto state = std::make_unique<MoStretch>(channels, max_bytes);
    mo_stretch::Scope scope(state->budget);
    state->engine = std::make_unique<Engine>(0x4d4f);
    state->engine->configure(channels, block, interval, false);
    *out = state.release();
    return 0;
  } catch (const std::bad_alloc &) { return 2; }
    catch (...) { return 4; }
}
extern "C" void mo_stretch_destroy(MoStretch *state) { delete state; }
extern "C" uint32_t mo_stretch_latency(const MoStretch *state, uint32_t *input, uint32_t *output) {
  if (!state || !input || !output) return 1;
  *input = state->engine->inputLatency();
  *output = state->engine->outputLatency();
  return 0;
}
extern "C" uint32_t mo_stretch_memory(const MoStretch *state, uint32_t *live, uint32_t *peak) {
  if (!state || !live || !peak) return 1;
  *live = state->budget.live;
  *peak = state->budget.peak;
  return 0;
}
extern "C" uint32_t mo_stretch_begin(MoStretch *state, const float *input, uint32_t frames, double rate) {
  if (!state) return 1;
  if (state->poisoned) return 3;
  if (state->started || frames != uint32_t(state->engine->inputLatency()) ||
      !std::isfinite(rate) || rate < 1.0/64 || rate > 64 || !finite(input, frames, state->channels)) return 1;
  try {
    mo_stretch::Scope scope(state->budget);
    state->engine->seek(Interleaved<const float>{input, state->channels}, frames, rate);
    state->started = true;
    return 0;
  } catch (const std::bad_alloc &) { state->poisoned = true; return 2; }
    catch (...) { state->poisoned = true; return 4; }
}
extern "C" uint32_t mo_stretch_process(MoStretch *state, const float *input, uint32_t in_frames,
                                       float *output, uint32_t out_frames) {
  if (!state) return 1;
  if (state->poisoned) return 3;
  if (!state->started || !output || !out_frames || out_frames > 4096 || in_frames > 65536 ||
      !finite(input, in_frames, state->channels)) return 1;
  // Even zero-input calls need a non-null view because upstream forms channel pointers.
  const float zeros[64] = {};
  try {
    mo_stretch::Scope scope(state->budget);
    state->engine->process(Interleaved<const float>{in_frames ? input : zeros, state->channels}, in_frames,
                           Interleaved<float>{output, state->channels}, out_frames);
    if (!finite(output, out_frames, state->channels)) { state->poisoned = true; return 4; }
    return 0;
  } catch (const std::bad_alloc &) { state->poisoned = true; return 2; }
    catch (...) { state->poisoned = true; return 4; }
}
#ifdef MO_STRETCH_FAULT_TEST
extern "C" void mo_stretch_fail_after(int64_t n) { mo_stretch::remaining = n; }
#endif
