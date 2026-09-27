# MusterOffice computation MCP — development adapter

The default `mo-mcp` is a thin local computation adapter. It shares SDK execution
and caller-file I/O with the CLI. It has no account, business authorization model,
database, durable task service, document library or UI. Existing persistent-host
code is available only through the separate [legacy entry](LEGACY.md).

This is a source development build, not a published installable release or full
presentation/Office/WPS acceptance. Remote transports, gateway attachments,
Skill/Plugin packaging and remaining presentation capabilities are still open.

The current dependency pin is official Rust SDK `rmcp 3.4.1`. The adapter
advertises and independently tests MCP `2026-07-28` and `2025-11-25` over stdio.
A current protocol version does not imply all optional extensions or installable
Plugin support; see [version and acceptance status](../../docs/implementation/mcp-protocol-status.md).

## Configure the caller's file channel

```sh
cargo build --manifest-path tools/mo-mcp/Cargo.toml --release --locked --offline
```

Set the MCP client's command to the built `mo-mcp` and pass one protected local
configuration file. The host creates and owns the three directories first:

```json
{
  "inputDirectory": "/absolute/caller/inputs",
  "outputDirectory": "/absolute/caller/outputs",
  "temporaryDirectory": "/absolute/caller/temporary",
  "computationSlots": 2,
  "controlSlots": 2,
  "exportWorker": {
    "path": "/absolute/caller/mo-export-worker",
    "sha256": "REPLACE_WITH_ACTUAL_MATCHING_WORKER_SHA256"
  }
}
```

The optional export worker is the matching pinned **export** worker, not the old
raster-worker interface. Without it, create/import/edit still work; capabilities
report no export renderer, and export requests return an explicit failure.
The temporary tree must be separate from input/output trees. These directories
and their ancestors remain protected and stable by the host throughout a call;
active outputs must not be externally renamed or mutated. This is a local file
bridge, not a sandbox against a hostile process controlling those directories.
No root path comes from an Agent tool argument. Symbolic links/reparse points,
path traversal, absolute paths and Windows device names are rejected in the
bridge. File names are ASCII portable leaf names, at most 128 bytes, without
trailing dots; subdirectories are not accepted as input names. A client without
access to the caller's file/attachment channel cannot use this local profile;
MusterOffice does not start an upload server to compensate.

The host also owns retention and crash recovery of final files. Temporary
execution leases allow bounded recovery during subsequent computations. They
do not constitute a persistent document/job store or a background service.

## Tools

- `mo_capabilities`: actual adapter limits, available export renderer identity,
  shared schema identifiers and explicitly incomplete presentation acceptance.
- `mo_schema`: the Rust-generated computation schema document and digest.
- `mo_presentations_compute`: create, import, atomic edit or export from caller
  files; it invokes the same `mo-embedded-sdk::execute` as the CLI.

Prepare an invocation JSON using `computation-invocation`, and an input manifest
of `{ "info": AssetInfo, "file": "resource.bin" }` entries. Each manifest file
name resolves only inside the configured input directory. Empty resources use
`[]`. The resource set must exactly match the request, with actual file length
and SHA-256 checked while staging immutable inputs. Nothing follows PPTX external
links or silently resolves fonts from the host operating system.

```json
{
  "invocationFile": "operation.json",
  "inputsFile": "resources.json",
  "outputDirectory": "new-result"
}
```

The output child directory must not exist. Successful structured output has
`outcome: "computed"` and a correlated result containing `requestId`,
`requestDigest`, `resultFile`, `resultByteLength`, `resultSha256`, `assets` and
`productCommitted: false`. Results also include a standard MCP resource link.
The actual computation receipt is in `result.json`; export additionally writes
`files.json`, the verified PPTX/PNG/resource bytes and `inspection.json`.
A failed operation sets MCP `isError: true` and returns the shared structured
failure. Ordinary failure removes this call's incomplete output; it never
replaces a previous result. Process termination or a lost response may leave
caller-owned files, which are not evidence of a business publication.

The adapter does not advertise Jobs or MCP Tasks. Each call completes, fails or
is cancelled within its execution lifetime. Persistent retries, document head
CAS, task recovery, user authority and publication remain product responsibilities.

## Resources, cancellation and bounds

`musteroffice://output/{directory}/{file}` reads caller output files, without a
library index or recursive listing. JSON full reads return text; binary reads
return blobs. Full reads are limited to 256 KiB. Larger files use the native file
channel or `?offset=0&length=262144` ranges. Range content is base64 bytes and
includes actual offset, returned length, current total length and chunk SHA-256.
The receiver checks the complete assembled file against its computation index;
a chunk hash does not prove an old whole-file digest or product authority.

Metadata invocations allow 65 MiB, with request/snapshot separately bounded by
the shared 32 MiB budgets. Inputs retain existing 128 MiB per asset, 512 MiB total,
32 MiB font and 1024-asset limits. MCP framing stays 4 MiB input/16 MiB response;
large documents stay in the explicit file channel. Capability discovery reports
these bounds separately. No claim of peak RSS or aggregate multiprocess disk
reservation follows from them.

Computation and resource-read pools each allow 1–8 active calls, without an
unbounded admission queue. Cancellation reaches computation and staged file I/O.
The blocking task keeps its permit and request identity until it has stopped
and attempted cleanup; dropping the async handler does not free its slot early.
EOF or transport failure immediately signals cancellation before protocol drain.
Transient spool registry contention waits cooperatively within 60 seconds;
cleanup has a separate bounded release window even after cancellation. Ordinary
OS file I/O is not claimed to have a hard real-time bound. Exhausted cleanup
bounds can leave lease-free orphans for the caller's next recovery attempt.

On Unix, stdio must be pipes and uses the existing nonblocking transport. The
Windows development fallback still requires native cancellable-I/O/lifecycle
acceptance. Both pinned protocol profiles are tested separately; protocol version
metadata and resource-not-found errors retain their respective wire behavior.

## Verification

```sh
cargo test --manifest-path tools/mo-mcp/Cargo.toml --locked --offline
cargo clippy --manifest-path tools/mo-mcp/Cargo.toml --all-targets --locked --offline -- -D warnings
python3 tools/mo-mcp/compute_check.py NEW_OUTPUT_DIR MO_MCP_BINARY MO_EXPORT_WORKER
python3 tools/mo-mcp/compute_lifecycle.py NEW_LIFECYCLE_DIR MO_MCP_BINARY
```

The independent protocol client checks both 2025-11-25 and 2026-07-28, real
create/edit/import/export, output and resource byte equality, shared schemas,
concurrent exports, file-scope negatives, large ranges and restart without a
service database. Lifecycle checks use a deliberately blocked private worker
for actual cancellation, EOF, slot retention and temporary cleanup; positive
rendering uses the real worker separately. Historical legacy tests remain under
[LEGACY](LEGACY.md), not relabelled as tests of the new default entry.
