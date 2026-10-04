# MusterOffice

**An office engine for AI agents, starting with editable presentations.**

English · [简体中文](README.zh-CN.md)

[Try Musterwork](https://app.musterwork.com) · [Template examples](#template-examples) · [Quick start](#quick-start) · [Agent integration](#connect-an-agent)

MusterOffice gives your agent or application structured operations to create,
inspect, edit, render and export presentations. A shared Rust core powers native
and WebAssembly execution, with thin SDK, CLI and MCP interfaces.

It already powers presentation workflows in [Musterwork](https://www.musterwork.com).
Use the hosted product to try the experience, or build this repository to integrate
presentation computation into your own application. The engine runs independently
of Musterwork and does not require a model provider or an account service.

## Try it

Open **[Musterwork](https://app.musterwork.com)** and ask the agent to create a
presentation. For example:

> Create a quarterly business review presentation with a revenue chart, a budget
> table and an action plan. Use the attached template and export an editable PPTX.

Download either template below and attach it as a starting point. The example
content is in Chinese; replace the text and fictional data with your own material.
The hosted application's account and runtime requirements apply.

## Template examples

These two original Musterwork templates demonstrate presentations imported and
rendered with MusterOffice. Each includes an editable PPTX and a full-deck preview
that displays directly on GitHub. Click a preview to view it at full size.

### Blueprint · Business Review / 蓝图 · 商务汇报

14 slides for business reviews, revenue analysis, budgets, roadmaps and decisions.
Includes **5 native charts, 6 native tables and 2 illustrations**.

[Download PPTX](examples/templates/business-blueprint/business-blueprint.pptx) · [Full-size preview](examples/templates/business-blueprint/preview.jpg)

[![Blueprint business review: all 14 slides](examples/templates/business-blueprint/preview.jpg)](examples/templates/business-blueprint/preview.jpg)

### Perspective · Research Report / 视野 · 内容报告

14 slides for research findings, audience analysis, customer journeys, comparisons
and action plans. Includes **3 native charts, 7 native tables and 2 illustrations**.

[Download PPTX](examples/templates/editorial-perspective/editorial-perspective.pptx) · [Full-size preview](examples/templates/editorial-perspective/preview.jpg)

[![Perspective research report: all 14 slides](examples/templates/editorial-perspective/preview.jpg)](examples/templates/editorial-perspective/preview.jpg)

Charts retain their data and embedded workbooks; tables retain editable cells.
The templates reference **Noto Sans SC**, which is not bundled. Install that font
in your editor to match the intended typography. Business data and cases are
fictional, and the illustrations are AI-generated. See the
[example guide and provenance](examples/templates/README.md).

## Quick start

### 1. Build the CLI

Install Git and [Rust through rustup](https://rustup.rs). The repository pins Rust
1.92.0 in `rust-toolchain.toml`; Cargo downloads the locked dependencies on the
first build. Run the following in a terminal:

```sh
git clone https://github.com/wanquanY/MusterOffice.git
cd MusterOffice
cargo build -p mo-cli --locked
```

The commands below use a POSIX shell on macOS or Linux. The CLI is written to
`target/debug/mo-cli`. This basic export path needs no native graphics libraries
or installed office application.

### 2. Export your first PPTX

Use the included two-slide document and its bundled image resource:

```sh
mkdir -p .codex-work/quickstart
target/debug/mo-cli pptx-export \
  fixtures/presentations/native-export/request.json \
  fixtures/presentations/native-export/resources.bin \
  .codex-work/quickstart/hello.pptx

target/debug/mo-cli pptx-inspect .codex-work/quickstart/hello.pptx
```

Open `.codex-work/quickstart/hello.pptx` in your presentation editor. This small
[fixture](fixtures/presentations/native-export/README.md) demonstrates editable
text, shapes, groups, images and connectors. Inspect or modify its
[request JSON](fixtures/presentations/native-export/request.json) to learn the
underlying document model. Choose a new output filename when running it again;
exports do not overwrite existing files.

You can inspect a complete template the same way:

```sh
target/debug/mo-cli pptx-inspect \
  examples/templates/business-blueprint/business-blueprint.pptx
```

### 3. Use structured operations

For application and agent workflows, `compute` uses the same invocation contract
as the Rust SDK and MCP. This example composes three slides from text and frames:

```sh
mkdir -p .codex-work/quickstart/temporary
printf '[]\n' > .codex-work/quickstart/inputs.json

target/debug/mo-cli compute \
  fixtures/presentations/compose/invocation.json \
  .codex-work/quickstart/inputs.json \
  .codex-work/quickstart/temporary \
  .codex-work/quickstart/composed

target/debug/mo-cli compute-schema computation-invocation \
  > .codex-work/quickstart/invocation.schema.json
```

The new `composed` directory contains `result.json` with the editable document
snapshot. Pass that snapshot explicitly to later edit or export invocations.
Resource manifests map declared assets to caller-supplied files. Full export with
PNG previews additionally uses a matching native export worker and explicit fonts.
See [native rendering setup](#native-rendering-and-playback) and the
[computation contract](contracts/generated/computation-invocation.schema.json).

## Connect an agent

Build the local MCP adapter, which has its own Cargo workspace:

```sh
cargo build --manifest-path tools/mo-mcp/Cargo.toml --locked
```

Create separate input, output and temporary directories, then save a
`caller-files.json` configuration using their **absolute paths**:

```json
{
  "inputDirectory": "/absolute/path/to/inputs",
  "outputDirectory": "/absolute/path/to/outputs",
  "temporaryDirectory": "/absolute/path/to/temporary",
  "computationSlots": 2,
  "controlSlots": 2
}
```

Register the server in your MCP client. For clients that use `mcpServers`:

```json
{
  "mcpServers": {
    "musteroffice": {
      "command": "/absolute/path/to/MusterOffice/tools/mo-mcp/target/debug/mo-mcp",
      "args": ["/absolute/path/to/caller-files.json"]
    }
  }
}
```

The server exposes `mo_capabilities`, `mo_schema` and
`mo_presentations_compute`. Start with capability and schema discovery. Your
agent's file tools must access the configured input/output directories. Add the
matching `exportWorker` path and SHA-256 to enable rendering and full export;
create, import and edit work without it.

Follow the [MCP guide](tools/mo-mcp/README.md) for worker configuration and tool
calls. For a packaged Skill and MCP integration, use the
[Agent package builder](tools/agent-package/README.md) and its
[presentation Skill](integrations/skills/musteroffice-presentations/SKILL.md).

## Embed in your application

| Integration | Start here |
| --- | --- |
| Rust SDK: create, compose, import, edit and export | [SDK guide](tools/sdk/README.md) and [native export example](tools/sdk/example/src/main.rs) |
| Browser / WebView playback with TypeScript and WASM | [Playback package](tools/playback-sdk/README.md) and [example](tools/playback-sdk/example.mjs) |
| Native headless playback | [Playback example](tools/sdk/playback-example/src/main.rs) |
| Interactive editing integration | [Editor client](packages/editor-client/README.md) |
| Versioned SDK / worker / WASM bundles | [Release assembly](tools/release/README.md) |

The caller provides document bytes, assets, fonts, cancellation and final output
storage. MusterOffice handles document computation and diagnostics. Continuous
playback belongs in the application runtime; it does not need one agent tool call
per frame.

### Native rendering and playback

Rendering uses the pinned HarfBuzz, Skia and image codec components. Prepare these
before building `mo-export-worker` or `mo-raster-worker`:

- [HarfBuzz setup](docs/implementation/harfbuzz-component.md)
- [Skia setup](docs/implementation/skia-component.md)
- [Image codec setup](docs/implementation/image-codec.md)

Set `MO_HARFBUZZ_LIB_DIR` and `MO_SKIA_LIB_DIR` to the verified native build
directories, then build the workers:

```sh
cargo build -p mo-export-worker -p mo-raster-worker --locked
```

Keep SDK, workers and WASM from the same verified build. Supply font bytes
explicitly; the core does not search system fonts or fetch external resources.

## Status and development

This is an early source release focused on everyday presentation workflows.
Creation, import, supported edits, native PPTX export, rendering and playback have
working implementations. Feature coverage varies by input and operation; full
PowerPoint/WPS interoperability and all advanced presentation features are still
in progress. Documents and spreadsheets are future domains.

There is currently no public package-manager release or prebuilt GitHub release.
Use the source instructions above. Native rendering currently has macOS and Linux
build profiles; Windows rendering support remains incomplete.

For TypeScript development, use Node.js 22+ and pnpm 10.2.1, then run
`pnpm install --frozen-lockfile` and `pnpm check:types`.
See [current verification](docs/implementation/current-verification.md),
[implementation progress](docs/implementation/progress.md),
[architecture](docs/architecture/overview.md) and [decisions](docs/decisions/README.md)
for detailed scope and evidence. Please include a minimal reproducer, your commit
and platform when reporting an [issue](https://github.com/wanquanY/MusterOffice/issues).

## Contributing

See the [contribution guide](CONTRIBUTING.md) for branches, pull requests, review and validation. Report vulnerabilities through the [private security process](SECURITY.md).

## Licensing

Original project source and the included original templates are licensed under
[Apache License 2.0](LICENSE). Third-party materials retain their own licenses and
attributions; see [third-party notices](THIRD_PARTY_NOTICES.md).
