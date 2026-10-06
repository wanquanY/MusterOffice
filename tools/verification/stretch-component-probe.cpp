#include "mo_stretch.h"
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <limits>
#include <vector>

static void require(bool value) { if (!value) std::abort(); }
int main() {
  require(mo_stretch_abi() == 1);
  unsigned failures = 0;
  // Every allocation point in construction fails in turn. Sanitizers observe
  // unwinding of partially initialized FFT/STFT/vector trees.
  for (unsigned n = 0; n < 200; ++n) {
    mo_stretch_fail_after(n);
    MoStretch *state = nullptr;
    auto status = mo_stretch_create(2, 5760, 1440, 32 << 20, &state);
    if (status == 0) { require(state); mo_stretch_destroy(state); break; }
    require(status == 2 && state == nullptr); ++failures;
  }
  require(failures > 10 && failures < 200);
  mo_stretch_fail_after(-1);
  MoStretch *state = nullptr;
  require(mo_stretch_create(2, 5760, 1440, 32 << 20, &state) == 0);
  uint32_t in_latency = 0, out_latency = 0;
  require(mo_stretch_latency(state, &in_latency, &out_latency) == 0);
  std::vector<float> input(in_latency * 2, 0.1f), output(256);
  require(mo_stretch_process(state, input.data(), 128, output.data(), 128) == 1);
  require(mo_stretch_begin(state, input.data(), in_latency, 1.5) == 0);
  require(mo_stretch_begin(state, input.data(), in_latency, 1.5) == 1);
  for (int block = 0; block < 1500; ++block) {
    for (size_t i = 0; i < input.size(); ++i) input[i] = float(.3*std::sin((i/2 + block*192)*.0576));
    require(mo_stretch_process(state, input.data(), 192, output.data(), 128) == 0);
    for (auto v : output) require(std::isfinite(v));
  }
  input[0] = std::numeric_limits<float>::infinity();
  require(mo_stretch_process(state, input.data(), 192, output.data(), 128) == 1);
  input[0] = 0;
  require(mo_stretch_process(state, input.data(), 192, output.data(), 128) == 0);
  uint32_t live = 0, peak = 0;
  require(mo_stretch_memory(state, &live, &peak) == 0);
  require(live <= peak && peak < 32 << 20);
  mo_stretch_destroy(state);
  // All finite input is admitted, but an internal non-finite computation is a
  // DSP failure and permanently poisons the instance.
  require(mo_stretch_create(1, 5760, 1440, 32 << 20, &state) == 0);
  std::vector<float> huge(in_latency, std::numeric_limits<float>::max());
  require(mo_stretch_begin(state, huge.data(), in_latency, 1.5) == 0);
  bool poisoned = false;
  for (int i = 0; i < 100; ++i) {
    auto status = mo_stretch_process(state, huge.data(), 192, output.data(), 128);
    if (status == 4) { poisoned = true; break; }
    require(status == 0);
  }
  require(poisoned);
  require(mo_stretch_process(state, huge.data(), 192, output.data(), 128) == 3);
  mo_stretch_destroy(state);
  std::printf("{\"allocationFailures\":%u,\"inputLatency\":%u,\"outputLatency\":%u,\"peakBytes\":%u,\"processedFrames\":192000,\"poisonedRejected\":true}\n", failures, in_latency, out_latency, peak);
}
