# Agent development package

One canonical [Skill](../../integrations/skills/musteroffice-presentations/SKILL.md)
and [portable identity](../../integrations/plugin.json) are assembled with two
explicit native executables. The generated Codex compatibility files refer to
the same Skill and MCP. There is no install hook, auto-download, database or UI.
Status and validation scope: [implementation](../../docs/implementation/agent-package.md).

Prepare matching native binaries using the documented component build inputs.
From the repository root, build a new development directory and optional archive:

```sh
python3 tools/agent-package/build.py \
  --mcp tools/mo-mcp/target/debug/mo-mcp \
  --worker target/debug/mo-export-worker \
  --output .codex-work/local-agent/musteroffice \
  --archive .codex-work/local-agent/musteroffice.zip
python3 tools/agent-package/build.py --verify \
  --output .codex-work/local-agent/musteroffice
```

Existing destinations are preserved. The archive contains `plugin.json` at its
root and records executable flags; the client's extractor must preserve them.
Header checks identify supported 64-bit Mach-O, ELF or PE targets but do not
replace actual loading on each platform. This stage verifies macOS arm64 only.
Package metadata is a development version; do not infer release readiness from
successful assembly. Fonts, media and caller documents are never copied in.

The generated [package guide](../../integrations/package-README.md) explains the
host-owned file configuration and removal/upgrade boundary. Registering or
installing a plugin is the client's operation, separate from this builder.
No user application configuration is changed by these commands.

Run packaging invariants and a real two-era MCP workflow with explicit binaries:

```sh
python3 -m unittest discover -s tools/agent-package -p test_build.py -v
python3 tools/agent-package/check.py \
  .codex-work/local-agent-check \
  tools/mo-mcp/target/debug/mo-mcp target/debug/mo-export-worker
```

`check.py` requires developer Python plus `jsonschema`. It drives the owned
two-page fixture through create/edit/export/import, verifies Resources bytes,
simulates replacement by a second assembly of the same version, rejects invalid
runtime/worker bindings, and runs with an empty executable search path. It is an
independent protocol client, not a test of an LLM selecting the Skill or a real
Agent application's plugin manager. `current.py` includes the packaging tests
in `lint` and the real workflow in `mcp-protocol`.

Portable manifest validation uses local copies of the official
[plugin schema](https://agent-plugins.org/schemas/1.0.0/plugin.schema.json) and
[MCP schema](https://agent-plugins.org/schemas/1.0.0/mcp.schema.json). Save them as
`plugin.schema.json` and `mcp.schema.json` in a developer-selected directory, then:

```sh
python3 tools/agent-package/validate.py \
  .codex-work/local-agent/musteroffice .codex-work/agent-plugin-schemas
```

The validator pins both SHA-256 values from the 2026-09-27 retrieval and disables
remote schema resolution. Schema or specification changes need a separate
review; runtime never downloads schemas. The platform's own validator and
actual installation/activation/upgrade checks remain distinct requirements.

`bundle-manifest.json` inventories all regular files except itself. It detects
accidental modification, missing/extra files and executable-bit loss; it is not
a signature. Recorded upstream notices are preserved, but complete transitive
license clearance and signed multi-platform distribution remain pending.
