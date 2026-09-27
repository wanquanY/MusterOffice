# Optional HTTP computation adapter

Enable `http` explicitly; default stdio and the embedded SDK do not include it.
This development endpoint supports MCP `2026-07-28`; older HTTP clients, remote
attachment staging and complete presentation acceptance remain open.

## Local use

Prepare the caller directories/configuration and matching export worker from
[README](README.md). Run from the repository root:

```sh
cargo build --manifest-path tools/mo-mcp/Cargo.toml --features http --bin mo-mcp-http --locked --offline
tools/mo-mcp/target/debug/mo-mcp-http /absolute/caller/config.json 127.0.0.1:0
```

The process prints its `/mcp` URL. Only numeric loopback addresses are accepted;
the launcher rejects browser Origin headers. SIGINT cancels and drains work.
Use the printed URL with matching request headers and metadata:

```sh
curl "$MUSTER_OFFICE_MCP_URL" \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/list' \
  --data '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"caller","version":"1"}}}}'
```

`tools/call` additionally needs `Mcp-Name: <params.name>`; `resources/read` uses
the resource URI as `Mcp-Name`. Follow protocol sentinel encoding for non-ASCII
header values. Clients support JSON and SSE and inspect JSON-RPC/tool outcomes,
not only HTTP status. No initialize, session ID or resumption token is needed.

Tools and file arguments match stdio. The product stages invocation/resources
in its input directory and receives output files. A URL alone is not an Agent
attachment channel; no upload server or document store is provided.

## Product embedding

In a Tokio runtime, construct `mo_mcp::http::Endpoint::new(&files, Options {
allowed_hosts, allowed_origins })`, retain it and pass authorized
`Request<axum::body::Body>` values to `handle`. Authenticate each request in the
product gateway **before** selecting a caller-specific endpoint. Explicit
Host/Origin validation is neither authentication nor CORS; empty origins reject
browser Origin headers.

The product owns TLS, connection limits, CORS, protected stable directory trees,
attachments, retention and business publication. On shutdown call `shutdown()`
and keep the runtime alive until blocking cleanup drains. Closing one response
cancels only that computation. See [limits and evidence](../../docs/implementation/http-mcp.md).

## Checks

```sh
cargo test --manifest-path tools/mo-mcp/Cargo.toml --features http --locked --offline
cargo clippy --manifest-path tools/mo-mcp/Cargo.toml --all-features --all-targets --locked --offline -- -D warnings
python3 tools/mo-mcp/http_check.py NEW_OUTPUT_DIR MO_MCP_HTTP_BINARY MO_EXPORT_WORKER
python3 tools/mo-mcp/http_lifecycle.py NEW_LIFECYCLE_DIR MO_MCP_HTTP_BINARY
python3 tools/mo-mcp/record_http_components.py --check
```

Python requires `jsonschema`; lifecycle fixtures use Unix signals and a blocked
worker. These are macOS/Unix development checks, not native Windows, public
gateway, Office/WPS or performance acceptance.
