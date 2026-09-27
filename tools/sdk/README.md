# MusterOffice Rust embedding SDK — development inputs

## Computation-only boundary

This development SDK provides caller-owned PPT computation. It does not include
accounts, authorization policy, durable jobs, databases or product UI. The
integrating product owns input access, final saving/publication and its task
lifecycle. Headless rendering/playback, advanced content, editable output and
validation remain the full project goal; this build has only partial feature
coverage and is not a replacement acceptance or Office/WPS certification.

`Presentation` supports in-memory creation, atomic editing, restoring supplied
snapshots, PPTX import and native export. `Inputs` borrows bytes or readers;
`ExportOptions` specifies document resources, fonts and delivery settings. The
product configures one native exporter with a pinned worker and an explicit
protected temporary directory. No JobStore, account context or business
committer is required for these calls.

`execute(Invocation, inputs, optional_exporter, cancelled)` is the shared direct
dispatch for typed callers and thin process adapters. `Invocation` carries the
same operation plus its explicit base snapshot for edits/exports. `Execution`
owns a correlated computation receipt and optional retained export bytes;
`into_parts()` transfers them without cloning a document. It does not accept or
create a durable job. Each request/snapshot retains the 32 MiB metadata budget.

```rust,ignore
let mut deck = Presentation::create(document, &cancelled)?;
let receipt = deck.edit(request_id, operations, &cancelled)?;
let candidate = deck.export(&exporter, export_id, options, &inputs, &cancelled)?;
// The caller reads candidate.assets()/candidate.open(id), saves where desired,
// and discards the temporary candidate after consuming its bytes.
```

The `operation` module now exposes `mo-presentation-operations` and the strict
`musteroffice.computation/1-draft` request, without `outputMode` or host fields.
The native worker protocol is `musteroffice.native-export/2-draft`; pair this SDK
with its matching pinned worker. Old request JSON is not silently reinterpreted.
The repository's legacy `mo-operation-service`, persistent host and opt-in
`mo-mcp-legacy` retain compatibility paths but are excluded from this SDK's
production dependency closure. The default local MCP now calls the shared SDK
through the caller-file adapter; remote/attachment profiles remain pending. See repository ADR 0007 and
`docs/implementation/kernel-boundary-roadmap.md` for scope and current progress.

## Existing development bundle

`playback::NativePlayback` adds a typed, caller-owned native sampler. Configure
an absolute raster-worker path, its SHA-256 and a per-call deadline; use
`prepare_author(PlaybackPrepareRequest, cancelled)` or
`prepare_source(PptxPlaybackPrepareRequest, source, fonts, cancelled)`. An existing
`Presentation` can call `prepare_author_playback(runtime, PlaybackOptions,
cancelled)` to bind its revision automatically. The source preparation method
borrows source/font `Content` only during preparation, so their readers/buffers
can be released before sampling. Both owners offer `info`, `sample`, `timing`,
`advance` and `dispose`. Each owner retains one worker and core plan, without
restarting or re-uploading inputs for every frame. Output is premultiplied sRGB
RGBA8 with unchanged core metadata and diagnostics. Continuous frames bypass
Agent tools; the product controls its own presentation clock, event history,
display and media services. Generation changes invalidate old event histories.

Input is transferred in bounded 64 KiB chunks; each command has one response,
with 32 MiB request metadata, 128 MiB source/font limits per bundle, 64 MiB
response metadata and 64 MiB pixel limits. These are protocol bounds, not total
RSS promises. Pixel dimensions, length, hash, alpha and sample binding are
checked before return. Transport failure, cancellation and timeout invalidate
and reap the owner; semantic computation errors preserve it when the core can
continue. `dispose` verifies clean shutdown and trailing-output absence; Drop
also terminates and reaps. The protected configured worker must not spawn
descendants retaining its pipes. Input readers/cancellation callbacks must
return promptly; they run on the caller thread and may be non-Send.

This exposes the existing author and source sampling profiles, not all planned
animation effects, transitions or media. Unsupported content keeps its typed
core diagnostic. Source/static resource coverage and author geometry coverage
remain different; no source-to-image fallback is applied. A changed document,
viewport, fonts or source requires a new owner. No application database, clock,
UI, font discovery or OS resource authorization is introduced.

The `mo-embedded-sdk` facade exposes the same document, computation, delivery and
native-export types as the kernel. It owns no database, queue, product identity
or commit authority. A separately pinned native worker performs graphics and
font computation. The product supplies authorized immutable assets, bounded
storage, cancellation, invocation fencing and atomic publication.

This local bundle is not a published release or a project license decision.
Registry package license declarations and included Unicode/ECMA notices are
retained; complete binary distribution notices and platform acceptance remain
required. The worker, fonts, media, browser bindings and product UI are separate.

The receiving build pins the SHA-256 of `sdk-manifest.json` from a trusted local
build record. Verify the complete directory before adding a relative path
dependency on `crates/mo-embedded-sdk`. The manifest does not authenticate itself.
`verify.py DIRECTORY --sha256 EXPECTED` verifies bytes and exact inventory; run
the verifier from a trusted source, not from an unverified download.

The package retains production Rust sources and required data. Cargo manifests
are normalized for the library-only dependency closure; development dependencies
and automatic test/example targets are excluded. Repository unit tests remain
the responsibility of the source build and are not claimed to run in this SDK.
There is no alternate implementation of document semantics or wire validation.

Platform-specific production dependencies retain their Cargo target predicates;
target-specific development dependencies and build scripts are not included.
The native exporter and worker must both support `--execution-lease-v1`. The
host supplies a protected spool root and calls `recover_spools` during startup
or maintenance; each prepare also performs bounded recovery. Live workers and
retained candidates hold OS leases. This does not retire application content
references or replace aggregate disk reservations. Legacy unleased directories
are not automatically deleted.

Build consumers outside this immutable directory. Set their target directory
outside it too. Cargo may need its already-locked registry packages; preparation
uses offline resolution and never silently fetches or selects new versions.
The example under `examples/native-export` is a consumer template: copy it to a
separate workspace and adjust only its SDK path dependency. It exercises actual
export and final retained-file inspection, without publishing a product result.

`examples/playback` is another standalone consumer template. Copy it outside
the SDK and point its sole SDK dependency at the pinned source bundle. Run it
with `worker sha256 author|source prepare.json samples.json new-output`, adding
`source.pptx fonts.bin` for source mode. It uses only the SDK plus serde_json,
writes actual frame metadata/RGBA files and disposes the worker. Samples are an
array of `{ "at": { "ticks": "1", "timescale": 2 }, "history": null }` values;
interactive profiles require complete explicit history. The example is a host
file adapter, not a separate calculation or storage service.
