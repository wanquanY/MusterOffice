# Typed operation and resource client

Development TS SDK layer over the existing Rust-generated host contracts. It
adds no runtime package dependency, document/layout algorithm, storage owner,
credential source, scheduler, automatic mutation retry, or product publication.
This is not the complete SDK, a released npm package, or a Musterwork adapter.

Build and test from the repository root:

```sh
pnpm run build:operation-client
pnpm run test:operation-client
node tools/verification/operation-client-native.mjs NEW_OUTPUT_DIR MO_HOST MO_RASTER_WORKER
```

The development ES module entry is emitted to
`.codex-work/operation-client/build/operation-client/src/index.js`. Generated
contract declarations are emitted alongside it for type resolution. No filesystem,
network or Node API is imported by this SDK; it can be injected into a JS host.
Browser execution and packaging still need their own acceptance.

## Host ports

`OperationPort` is already bound to the caller's trusted host authority. Its
adapter validates wire values with the generated contracts before returning
typed responses. A TypeScript cast or this client's correlation checks are not
a substitute for wire validation. The client never accepts principal, scope,
credentials, database/executable paths or publication fences as tool authority.
Ports own transport timeouts, bounded request admission, response draining after
a local waiter leaves, and actual resource permissions.

`OfficeClient.dispatch/result/submit/getJob/cancelJob/waitForJob` keep the
existing `HostRequest/HostResponse` and job receipts. `result` extracts an expected
result kind and preserves host failures. `submit` never converts a failed
transport into a new request. Retrying an uncertain mutation must use the same
original business requestId and payload. Callers keep the request immutable while
the port is using it. `waitForJob` has a finite budget and follows the host poll
hint; it creates no persistent queue or subscription.

`CallOptions.signal` ends local waiting. A submitted operation may still finish;
an abort/timeout never sends `cancelJob` or `cancelUpload`. Explicit business
cancellation is a separate host operation. Late promises are observed so a
rejection after local abort cannot become an unhandled rejection.

## Binary resources

`ResourceClient.upload` takes a fixed descriptor and an immutable `ByteSource`.
It uses the host's acknowledged offset/chunk size, transfers one bounded chunk
at a time, and asks the host to seal and verify the actual stored bytes. The
host decides quotas and expiry; the client has its own byte/chunk budget. Retry
with the same upload requestId after a lost acknowledgement: the host's committed
offset decides where to resume. No implicit upload cancellation or deletion is
performed on interruption. Offsets remain bigint locally and canonical decimal
strings on the host wire, including offsets above Number's exact integer range.

`copyAsset` accepts an optional expected descriptor from an immutable bundle.
It compares that pin before creating private output. The injected `CandidateStore`
provides a private, backpressured sink. The client seals it, hashes its final
stored bytes through an injected incremental SHA256 implementation, and releases
the verification reader. It returns a private reference only after length/hash
verification. Empty assets are verified too. This is byte integrity, not evidence
that Office/WPS editing, layout or playback has passed.

The store owns setup cleanup if `begin` fails. Once `begin` succeeds, failure
triggers `discard`; sealed readers always receive `release`. Private write,
seal and read operations settle before cleanup, preventing a late operation
from racing deletion. Host implementations must therefore make these operations
bounded/cooperatively cancellable; this SDK cannot forcibly interrupt an
arbitrary callback. Cleanup failures are retained alongside the original error.
Releasing a verification reader must not delete the sealed candidate bytes.

Musterwork must supply its own existing owner/authorization and private Content
Store adapters, then atomically validate fence, pins and complete bundle before
committing. This client does not add a second SQLite store to Embedded mode.
The native driver under `tools/verification/operation-client/` is an independent
test harness for Standalone mode, not the product's production transport.
