# Bind the actual tools and caller file channel

Use the MusterOffice connection supplied by the host. Tool names may have a
host prefix; identify their provider, not just the short name. Current tools:

| Tool | Arguments | Result |
| --- | --- | --- |
| `mo_capabilities` | `{}` | Implemented operations, contracts, resource limits, exporter identity and acceptance flags |
| `mo_schema` | `{ "id": "computation-invocation" }` or another advertised schema ID | Authoritative schema document |
| `mo_presentations_compute` | `invocationFile`, `inputsFile`, `outputDirectory` | Structured computed/failed result and an output resource link |

The host configures input/output/temporary directories before connecting. Use
its authorized file or attachment tools to place JSON and resource bytes in the
input directory; the MCP server does not upload attachments or resolve URLs.
If no input channel exists, identify the missing host binding before claiming
the task can run. Plugin installation does not create that channel implicitly.

Input arguments are portable ASCII leaf names, at most 128 bytes. Supply a new
output child name for each computation; never overwrite or mutate a previous
result while it is being read. Example tool arguments:

```json
{"invocationFile":"operation.json","inputsFile":"inputs.json","outputDirectory":"revision-002"}
```

`operation.json` follows `computation-invocation`. The input manifest is an array
of `{ "info": AssetInfo, "file": "resource.bin" }` entries. AssetInfo identifies
the asset, media type, SHA-256, decimal byte length and `bytesSha256` verification.
Use `[]` when there are no input resources. Match the resource closure required
by the chosen operation; do not guess hashes, worker identities or font bytes.

The tool result has `outcome: computed` or `failed`; on failure inspect its code
and diagnostics. A computed result identifies `resultFile`, byte length and hash.
Read the actual receipt using the host file channel or MCP Resources. Outputs
are under `musteroffice://output/{directory}/{file}`. Full resource reads are
bounded to 256 KiB; larger files use the host file channel or explicit offset and
length chunks. Assemble outside model context and verify the whole-file hash.

HTTP uses the same tools but currently requires 2026-07-28 request metadata.
Stdio supports 2025-11-25 and 2026-07-28. Let the MCP client handle wire framing;
HTTP closing cancels that request, whereas stdio uses cancellation notifications.
Do not invent persistent Jobs/Tasks endpoints. A lost response is not a reason
to publish or overwrite output: inspect the caller's existing result first.

Only use CLI fallback when the host has provided its verified binding. The
shared entry is `mo-cli compute <invocation> <inputs> <temporary> <new-output>
[worker worker-sha256]`; schema discovery is `mo-cli compute-schema <schema-id>`.
Use the same inputs/profile/revision as MCP. Do not download an unrelated binary
or replace a failed computation with an alternate document engine.
