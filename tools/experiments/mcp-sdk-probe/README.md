# Native MCP SDK selection probe

This standalone developer experiment checks the pinned official Rust SDK against the kernel's actual schema computation. It is not a production MCP server, office tool catalogue, scheduler, resource transport, plugin or E0-6 acceptance result. It has its own locked Cargo workspace; the computational workspace does not acquire MCP or Tokio dependencies.

The single `mo_probe_schema` tool takes an existing typed `SchemaId` and returns the real `SchemaDocument`. Its input is generated from the Rust query type and its output schema from `mo-operation-service`. No office algorithm or document authority is reimplemented.

Run from the repository root:

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked --offline \
  --manifest-path tools/experiments/mcp-sdk-probe/Cargo.toml \
  --target-dir .codex-work/mcp-native/formal-target
python3 tools/experiments/mcp-sdk-probe/check.py \
  .codex-work/mcp-native/protocol-new \
  .codex-work/mcp-native/formal-target/release/mo-mcp-sdk-probe
```

The locked dependencies must be available in Cargo's local cache for offline reproduction. The client uses the frozen operation-discovery stage's ten schema documents as independent expected outputs. It launches separate real stdio processes for `2025-11-25` initialize and `2026-07-28` per-request metadata/discovery, checks ten schema results in each, rejects unknown IDs/extra authority fields/unknown tools, and verifies an over-budget input terminates without a success response. The modern session also rejects unsupported protocol versions and missing required metadata. Every request and response is saved in a fresh directory.

The default upstream async stdio reader buffers complete lines without a byte limit. This experiment instead configures the SDK's framed codec with a 4,096-byte input limit; that limit is sufficient for schema identifiers, not a production document-request budget. Output sizes, queued messages, in-flight handlers and the full runtime memory budget require their own production implementation. Bad frames close this probe connection; full protocol error/recovery policy remains part of the final adapter.

No HTTP, OAuth, client transport, default macro feature or binary base64 feature is selected. The SDK's `server` feature still includes its own transitive dependencies, which must be measured and reviewed before adoption. The [component record](../../../components/rmcp/component.json) preserves the actual upstream archive identity and full mixed-license transition notice. It does not assert release clearance.
