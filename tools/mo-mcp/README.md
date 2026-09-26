# Native MCP development adapter

This standalone Rust workspace connects standard MCP tools/resources to the
existing native `StandardHost`. It is an implementation of the current draft
operation contract, not a released SDK or a complete presentation acceptance.
No MCP/Tokio dependency enters the computational workspace or WASM kernel.

Build from the repository root:

```sh
cargo build --manifest-path tools/mo-mcp/Cargo.toml --release --locked
```

Configure the MCP client's server command as `tools/mo-mcp/target/release/mo-mcp`
with `/absolute/path/operator.json` as its argument.

The installation supplies a protected configuration file. Paths must be
absolute; permissions have no implicit all-access default. For example:

```json
{
  "database": "/absolute/path/office.sqlite",
  "principal": "agent:example",
  "scope": "workspace:example",
  "permissions": ["create", "edit", "export", "readDocument", "readJob", "cancelJob", "writeAssets", "readAssets"],
  "workers": 2,
  "controlSlots": 2,
  "computationSlots": 2,
  "previewWorker": {
    "path": "/absolute/path/mo-raster-worker",
    "sha256": "REPLACE_WITH_ACTUAL_WORKER_SHA256"
  }
}
```

Omit `previewWorker` to run without export previews. The capability catalogue
then reports the corresponding operation unavailable. Configuration paths,
principal, scope and grants are never taken from tool arguments or resource URIs.
Filesystem permissions protecting this file/database remain installation duties.
On Unix, the MCP client must launch the server with stdin and stdout connected
to pipes. Checked nonblocking pipe I/O owns those streams; terminal/regular-file
server streams are rejected. The binary data subcommands retain ordinary file
redirection. Windows still uses the previous development I/O implementation;
its cancellable I/O and native lifecycle acceptance remain unfinished.

## Tools and resources

The adapter provides 13 named tools projected from the Rust `HostRequest`
contract. Create/apply/export carry `request` with its `requestId`, profile and
typed action; other tools use the corresponding generated request fields. Their
schemas and structured results use the same definitions as native dispatch.
Tools are sorted and filtered by configured permissions. A job remains in the
same SQLite owner across disconnect/reconnect; only `mo_jobs_cancel` requests
business cancellation. An identical retry must retain its original `requestId`.

`musteroffice://adapter/limits` distinguishes protocol budgets from larger host
budgets. `musteroffice://kernel/schemas/<schema-id>` exposes ten canonical schema
documents. Successful authorized asset/export results include resource links.
Asset namespaces bind scope/principal within the originating server; IDs and
hashes do not confer access. Every read goes through the host authorization.

Assets up to 1 MiB are returned as standard MCP base64 blobs after byte digest
verification. Larger resources return JSON transfer descriptors. The same
binary supports an operator-invoked data channel with exactly the configured
permissions and database; binary bytes do not pass through model arguments:

```sh
mo-mcp /absolute/path/operator.json append UPLOAD_ID OFFSET < authorized-input-chunk
mo-mcp /absolute/path/operator.json read-asset ASSET_ID OFFSET LENGTH > candidate-output
```

Append chunks are at most 256 KiB. After upload, call `mo_assets_seal`; after
download, check process exit status, full length and SHA256 before publishing a
file. A failed download can leave partial stdout. The protocol does not execute
these commands itself or authorize arbitrary input/output paths.

## Execution boundaries

The current stdio profile bounds input lines to 4 MiB, serialized responses to
16 MiB (excluding newline), active/unwritten requests to eight, and active
notifications separately to eight. Each bridge pool allows 1–8 blocking host
calls; native document jobs have their separate 1–16 worker budget. Unix pipe
I/O needs no blocking worker. Non-Unix development stdio retains two additional
blocking slots. Response writes have a ten-second deadline.
These are individual limits, not a proof of peak RSS or a global multiprocess
CPU budget. The catalogue/resource response costs are included in these limits.

An output permit remains held through flush. A cancelled stdio request remains
accounted for while its handler lives; its ID is then retained against stale
response aliasing, with a bounded history. Cancellation after response dispatch
does not release an unwritten response. A dropped async waiter does not free its
native permit early or undo a durable operation. Service exit drains blocking
control calls and joins native workers; queued jobs remain stored.

Malformed UTF-8/JSON and invalid request/parameter envelopes receive standard
bounded errors; valid notification envelopes never receive a response. One
additional protocol-error slot waits for its complete write before reading the
next input frame, and survives interruption of the SDK receive future. Oversize
or unterminated frames, unsolicited responses, aliases of active/cancelled IDs,
admission exhaustion and output failures terminate the connection with nonzero
exit. Both supported stdio versions use client request cancellation; it does not
cancel durable business work. HTTP transport, MCP Tasks, extension negotiation,
broader client interoperability, Windows lifecycle guarantees, total resource
budgets and distribution remain required work. This profile does not claim
complete MCP conformance or E0-6 acceptance.

## Verification

```sh
cargo test --manifest-path tools/mo-mcp/Cargo.toml --locked
cargo clippy --manifest-path tools/mo-mcp/Cargo.toml --all-targets --locked -- -D warnings
python3 tools/mo-mcp/check.py NEW_OUTPUT_DIR MO_MCP_BINARY MO_RASTER_WORKER
python3 tools/mo-mcp/check_transport.py NEW_TRANSPORT_OUTPUT_DIR MO_MCP_BINARY
python3 tools/mo-mcp/check_lifecycle.py NEW_LIFECYCLE_OUTPUT_DIR MO_MCP_BINARY
python3 tools/mo-mcp/check_cancellation.py NEW_CANCELLATION_OUTPUT_DIR MO_MCP_BINARY
```

`check.py` uses independent stdio framing, JSON Schema checks, the self-owned
two-page fixture, actual worker exports, resource/binary byte equality, SQLite
inspection, reconnect and denied-scope checks. It requires Python `jsonschema`.
Unit tests cover bounded queues, slow output, cancellation ordering and a real
SQLite-blocked call whose async waiter disappears. Historical failure artifacts
remain separate from later successful runs. The lifecycle client keeps stdin
open while waiting for failed processes, and cancellation checks use actual SDK
notifications while a SQLite lock holds a native call. See the
[original implementation](../../docs/implementation/mcp-stdio.md) and
[recovery and lifecycle record](../../docs/implementation/mcp-recovery.md).
