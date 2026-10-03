# WASM playback SDK — development package

This is a typed adapter over the existing Rust playback owners and the same
Skia/HarfBuzz WASM components. It has no accounts, persistent jobs, storage, UI,
clock, implicit fonts or network downloads. Current author/source profiles are
partial; full effects, transitions, media, SmartArt/math, Office/WPS and product
replacement acceptance remain required.

Run computation in a Worker owned by the integrating product. A development
bundle supplies the JavaScript/declarations, Rust WASM and graphics/text WASM.
It has no runtime npm dependencies. The host verifies the pinned bundle manifest
and explicitly loads/compiles its three code files before initialization:

```js
import {createPlaybackRuntime} from './musteroffice-playback/index.mjs';
const runtime = await createPlaybackRuntime({
  kernel: await WebAssembly.compile(kernelBytes),
  raster: await WebAssembly.compile(rasterBytes),
  text: await WebAssembly.compile(textBytes),
});
const owner = runtime.playback.prepareSource(prepareRequest, {
  source: pptxBytes, fonts: explicitFontBytes,
  decoder: runtime.raster, shaping: runtime.shaping,
});
// Original input arrays can now be released or transferred away.
try {
  const frame = owner.sample({ticks: '1', timescale: 2}, runtime.raster, history);
  // The product displays or transfers frame.pixels and owns the returned info.
  owner.dispose();
} finally {
  owner.close();
}
```

`prepareAuthor(request)` accepts the generated author prepare contract instead.

`prepareDeliveryInputs({delivery, width, height?}, contents)` derives deck-ordered source
page requests (uniformly fitted inside both bounds when height is present), exact PPTX/font asset identities and the explicit font manifest
from a transported delivery. It runs the existing full byte/context inspection
once and creates no playback owner or pixels. The portable content channel is
bounded to 128 MiB; native receivers can instead reuse
`ReceivedDelivery::playback_inputs` over their already inspected asset readers,
without packing or reopening content. Fonts are shared across the returned page
requests, not copied into every page entry. The returned document revision is
provenance; source playback's `binding.revision` must use `inputs.source.sha256`.
Construct the existing `prepareSource` page with profile
`drawingml-resource-page-q32-v1-draft`, the selected page's `request` fields,
and `inputs.fonts`. Supply the selected source/font bytes through their existing
binary channels. Actual source preparation still checks supported timing and
resource semantics; successful input derivation is not a playback quality claim.

`sample(at, raster, history = null)`, `timing()`, `advance(generation)`, `info`,
`dispose()` and idempotent `close()` cover the retained computation lifecycle.
`info` is a detached copy; changes to it cannot change a live owner's binding.

`AuthorPlayback.resize(viewport)` and
`SourcePlayback.resize(viewport, {source, decoder})` preserve the playback
binding, logical time and consumed event-prefix cache. Source resize requires
exact original PPTX bytes again, but no font bytes or shaping component. It
re-admits static image grids for the new scale while retaining text paths and
the source/timing index; animated image geometry continues to require exact grids.
A candidate is certified before publication; invalid input, cancellation or a
failed decoder leaves the prior viewport/resources usable.

For a window or DPR change, use `resizeToFit(width, height)` for author playback,
or `resizeToFit(width, height, {source, decoder})` for source playback. The host
supplies its physical pixel box; Rust derives exact uniform scale and rounded
allocation dimensions from the original document, applying the 8192-axis and
16-megapixel limits. No dimensions are inferred from previously rounded frames.
The wire operation is `resizeToFit`, with the same binding and viewport revision
precondition as explicit resize. For initial preparation, pass optional `height`
to `prepareDeliveryInputs` to fit before decoding the first page.

Every accepted resize increments `info.viewportRevision` (u32, never wraps).
The wire request requires `expectedViewportRevision`; stale requests fail with
`VIEWPORT_CONFLICT`. `planId` remains a content identity and may return to its
old value after A → B → A. A separate private owner epoch rejects prepared
frames from every earlier viewport, including this ABA case. `Frame.viewportRevision`
is captured by the thin client under its exclusive lease. The host must also
fence messages already delivered to its queue against the current owner and
viewport revision before displaying them. Close/take any stepped execution
before calling resize, then sample the same time/history again. The product
still owns DPR mapping, resize coalescing, clocks and fullscreen gestures.

For cooperative scheduling, `beginSample(at, raster, history)` returns an explicit
`PlaybackExecution`. The product chooses when to yield and when to cancel:

```js
const execution = owner.beginSample(at, runtime.raster, history);
try {
  while (!hostCancelled()) {
    if (execution.step(7)) {
      const frame = execution.take(); // Validated complete pixels, exactly once.
      publishIfCurrent(frame);        // Product owns publication and generation.
      break;
    }
    await hostYield(); // Product-supplied scheduling, outside the Rust callback.
  }
} finally {
  execution.close(); // Cancel/release before owner.advance/dispose/close.
}
```

The execution exclusively leases its playback owner until take/close; attempts
to sample, resize, advance or dispose that owner meanwhile return BUSY. A healthy early
close permits a new sample. Component faults quarantine the component and close
the playback owner; the product must terminate that Worker. `step` accepts
1–4096 work units, not a time budget. After rasterization, `step` snapshots the
component reply into owned Rust memory, then validates premultiplication and
hashes at most 16 KiB per validation work unit. `complete` becomes true only
after this validation; `take` still checks the owner/generation, hashes the frame
commands, serializes metadata and copies the final pixels out. Observed invalid
replies fail and quarantine immediately, including before a later close.
Preparation, individual Skia primitives, allocations/copies, command hashing and
final metadata serialization still include synchronous work. This API does not claim
bounded cancellation latency or immediate physical memory reclamation.
Pass generation/ticks as canonical decimal strings, never floating-point numbers.
The Rust core validates requests, resources, profiles and component pixels;
this adapter validates response correlation, dimensions and ownership. It is
bound to trusted matching WASM exports, not an arbitrary JSON transport.

Calls are synchronous. No AbortSignal or timeout is presented as capable of
interrupting a synchronous WASM call. The host terminates its Worker for hard
cancellation/deadline, waits for that termination where supported, rejects late
results using its current operation/generation, and recreates a runtime when
needed. Reentrant sample/control/close calls on the same owner are rejected
before entering Rust. No hidden background thread or frame queue is created.

Normal `PlaybackComputationError` preserves the typed Rust diagnostic and plan.
For stepped completion, `invalidatesBackend: true` additionally requires
isolation-unit disposal; the adapter invalidates the raster and releases its owner.
An input/bridge/trap/invalid response releases the owner; further calls fail
with CLOSED. Dispose always frees even if its response is invalid. Frame Rust
allocations are consumed exactly once; failed metadata reads release them.
Cleanup failures remain observable alongside the original failure. Use close
in finally; JavaScript garbage collection is not the release contract.

Components remain caller-owned. A bad external component reply can leave the
immutable core plan usable with a healthy component. If an actual supplied
RasterComponent/ShapingComponent becomes invalid, follow its isolation contract:
terminate the host Worker and prepare again in a new one. The host example marks
this failure fatal; Node can await termination, browsers cannot acknowledge it.
`runtime.createRaster()` and
`createShaping()` create additional healthy instances, not recovery authority
for a poisoned Worker. Calling
createPlaybackRuntime twice in one imported module is rejected, including after
failed initialization; use a new Worker for a new Rust runtime. Do not initialize
the bundle's internal kernel binding separately.

Requests have a 32 MiB JSON budget; source and font arrays each have a 128 MiB
limit and must use independent ArrayBuffers, not SharedArrayBuffers. Metadata
and pixels each have a 64 MiB response bound. These are protocol limits, not
RSS promises. The existing wasm-bindgen/component bridges copy some data; this
adapter adds no second frame copy or per-frame source/font upload. It does not
promise zero copy, 60 FPS or a desktop installation size.

The package also includes receiving-product examples:
`examples/node-worker.mjs` and `examples/browser-worker.mjs`. The browser exports
`openBrowserWorker({kernel, raster, text})` for explicit compiled modules; its
dedicated module Worker invokes this same SDK. The host serves immutable trusted
JS, supplies input buffers, and owns clock/display/lifecycle. `call(command,
transfer, timeoutMs, signal)` permits one outstanding call. Abort/deadline closes
the host, calls terminate and prevents late-result reuse. Browser termination
does not acknowledge completion and busy execution may continue briefly; this
is not an immediate CPU/memory reclamation guarantee. Bounded computation and
full cancellation performance acceptance remain pending.

The host examples also accept `beginSample`, `stepSample`, `takeSample` and
`cancelSample`. Cancellation between steps is acknowledged after explicit frame
cleanup. This private example envelope is not a second public Agent protocol.
An integrating product can drive `PlaybackExecution` directly inside its Worker
to choose its scheduling granularity without an IPC round trip for each step.

The bundle is for local development, with releaseCleared false. Supplied notice
records do not constitute a completed binary distribution license audit. Actual
browser/platform acceptance, product Viewer/Player, complete capabilities and
performance/size measurements are separate from SDK integration tests.


## Editing page computation

The same verified runtime provides `editor.restore(snapshot)` for an authorized
saved snapshot, retaining its kernel revision, and `createEditorPage()` for an
independent editor page owner. The page supports document inspection, preparation,
text interaction and picking through the public SDK; consumers do not import the
private WASM binding. Pass `runtime.raster` and `runtime.shaping` as explicit
computation ports and close every page when finished. Multiple page owners share
code and component instances, but each keeps its own current view and lifetime.
These methods perform computation only; the product owns authorization, durable
history, scheduling and save confirmation. See [editor API](../editor-client/README.md).
