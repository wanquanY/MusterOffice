# Browser project audio computation

`mo-video-mix-wasm` is the independent synchronous project mixer component. It
uses the existing `mo-video-audio::AudioMixer`, compiler and exact time model.
It does not include authoring operations, codecs, file/network access, device
clocks or authorization. `tools/video-mix-sdk/build.py` builds a fully inventoried,
immutable development delivery; its external manifest SHA is the consumer pin.

`ProjectAudio.open(documentJson, expectedDigest, describe, read)` validates and
compiles exactly one immutable revision. `AudioMixer::from_shared` owns an `Arc`
to that revision and preserves bounded DSP caches across queries. Native callers
can continue borrowing their compiled document. Neither mode leaks the document,
recompiles per block nor creates a self-referential allocation.

The synchronous capabilities have these contracts:

- `describe(bindingJson)` returns the shared `AudioResourceDescription` JSON.
  The descriptor binds the complete resource/content/stream descriptor, exact
  sample zero/range and explicit speaker layout. `MoviePcm.description()` emits
  it from the shared movie planner. Host JS must not infer these timing values.
- `read(windowJson)` returns precisely the requested interleaved f32le bytes.
  Input includes the complete binding, exact decimal sample endpoints, frame
  count and admitted byte count. Wrong length/type, non-finite PCM, stale binding
  or incorrect lattice fails. The core admits the length before copying bytes.
- Null/undefined is a pending source, with the exact binding and (for a read)
  requested range in the structured error. The asynchronous host may prepare or
  open the resource and retry. Promises are not accepted by these synchronous
  capabilities. Failed queries never return partially mixed success.

Output is stereo with explicit mono duplication. Other layouts require a future
explicit mapping capability. Each public query is 1..4096 frames. Internal source
windows are bounded to 65536 frames / 4 MiB, active PCM to 64 MiB, retained kernels
to 32 MiB and retime plans to 16 MiB. These are computation charges, not a promise
about total linear WASM memory, JS/codec memory or host source caches. The host
must separately own capacity, admission, deadline and Worker termination.

`info()` returns the pinned digest, exact project frame/sample counts and output
format. `stats()` reports only the last successful query; errors clear it.
`clear()` discards DSP caches without rebuilding temporal indexes. Random reads
never move a player clock. The host must keep the existing AudioWorklet and
actual device output clock as the playback authority.

Non-unit preserve-pitch currently requires an actual derived-audio host and
returns `unsupported_capability` without that capability. It is never replaced
by pitch-changing resampling. The shared stretch implementation exists but its
production Browser resource preparation/playback wiring is separate remaining
work. This component alone does not establish product UI or full-plan acceptance.

## Verification

`mo-video-audio` covers borrowed/shared ownership parity and final-owner release,
exact resource metadata and existing DSP regression. The public-delivery check is:

```sh
node tools/verification/video-mix-sdk-check.mjs \
  target/debug/mo-video-conformance DELIVERY REPORT.json
```

It compares the actual public WASM against native computation using the common
fixture (overlap, rate, reverse, gain/fade, nested sequence and random windows),
and checks pending capabilities, malformed payloads, stale descriptions, NaN,
Promise rejection and successful retries. The capability fixture is explicitly
test-only; actual media, persisted packs and device playback require Product
integration evidence.
