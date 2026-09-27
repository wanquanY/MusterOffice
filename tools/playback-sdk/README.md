# Local WASM playback package

`build.py` compiles the thin TS facade and the existing component adapters, then
copies explicitly prepared Rust wasm-bindgen **web** output, Skia and HarfBuzz
modules into one local development directory. It does not compile C++, fetch
dependencies, install a plugin or publish a package. Current functionality and
host responsibilities are described in [the SDK README](../../packages/playback-client/README.md).

```sh
python3 tools/playback-sdk/build.py --output .codex-work/playback-sdk-package \
  --wasm-dir /absolute/path/to/matching-wasm-bindgen-web-output \
  --raster-dir /absolute/path/to/pinned-skia-output \
  --text-dir /absolute/path/to/pinned-harfbuzz-output \
  --archive .codex-work/playback-sdk-package.tar.gz
python3 tools/playback-sdk/build.py --verify .codex-work/playback-sdk-package \
  --sha256 SHA256_FROM_TRUSTED_BUILD_RECORD
```

All output paths are new; existing packages/archives are never overwritten.
The manifest binds every file's bytes and relative path. Its external digest
must be pinned by the receiving build; the manifest does not authenticate itself.
Keep the package immutable while it is loaded. Tar headers and gzip timestamps
are normalized for reproducibility. `releaseCleared: false` records incomplete
release/platform/license acceptance; it does not change capability diagnostics.

The package exports ESM plus `.d.mts` declarations usable with ES2022. The raw
wasm-bindgen declarations separately use `ESNext.Disposable`; a consumer directly
importing that internal binding must include its required library types. The
public facade neither imports those declarations nor requires a Symbol.dispose
polyfill. All Rust owner cleanup is explicit through dispose/close.

The included `examples/node-worker.mjs` is a receiving-product example. Copy or
review it as host code, then call `openWorker(packageDirectory, manifestPin)`.
It validates package bytes, loads the three explicit code modules, serializes
one call at a time, transfers frame buffers and terminates its actual Worker
before rejecting on transport/deadline failure. It contains no office semantics,
durable job service or UI. Its private command envelope is not a new public
cross-process protocol. The product supplies its own input/output and lifecycle.

`examples/browser-worker.mjs` provides `openBrowserWorker(compiledModules)` for
a browser host. Its sibling `browser-runtime.mjs` is a dedicated module Worker;
the Node and browser examples share `dispatch.mjs` to invoke the same SDK owners.
The host build validates and serves immutable package JavaScript at its trusted
origin; the caller supplies the three compiled WASM modules. This example is
not a network package installer or a substitute for the host's code integrity
policy. See [browser verification and limitations](../../docs/implementation/browser-playback-sdk.md).

```js
import {openBrowserWorker} from './playback/examples/browser-worker.mjs';
const worker = await openBrowserWorker({kernel, raster, text}); // WebAssembly.Module
try {
  await worker.call({operation: 'prepare', kind: 'source', request,
    source: pptxBytes, fonts: fontBytes}, [pptxBytes.buffer, fontBytes.buffer]);
  const frame = await worker.call({operation: 'sample', at: {ticks: '1', timescale: 2}},
    [], 30000, cancelSignal);
  // Product code displays frame.pixels, then releases it.
  await worker.call({operation: 'dispose'});
} finally {worker.close();}
```

One call is allowed at a time. Input buffers are transferred and detached before
sampling; frame buffers transfer back. An active abort, deadline, fatal component
failure or transport error closes this host permanently and invokes terminate.
An already aborted signal rejects without stopping a healthy idle Worker; a
settled call removes its signal handler. A new Worker is required after failure.
Browsers provide no termination-completion Promise. Logical rejection and
suppression of late results do not prove immediate CPU/RSS reclamation; Chromium
can allow busy code to run until delayed forcible termination. This example does
not meet or redefine the full cancellation performance gate. Native process
isolation, cooperative bounded computation and per-engine acceptance remain
separate requirements. No UI, business storage or SharedArrayBuffer is required
for the ordinary browser computation path.

For cooperative execution, the shared dispatcher accepts `beginSample` (at and
optional history), `stepSample` (workUnits), `takeSample` and `cancelSample`.
The host sends cancel between completed step calls and waits for its cleanup
acknowledgment. Dispose also closes an active frame before disposing the plan.
These are bounded in-memory computation objects, not durable jobs. Direct SDK
consumers may instead drive `beginSample` inside the Worker and supply their own
yield/cancel handling, avoiding per-step IPC. An in-progress synchronous primitive
still cannot observe a queued cancel; the full cancellation gate remains open.

Build dependencies remain the repository's fixed Python/Node/TypeScript tools.
Runtime is JavaScript/WebAssembly and platform Worker facilities only. Fonts and
documents are caller inputs and are never included automatically. The preserved
notice records are development evidence, not a completed distribution audit.
