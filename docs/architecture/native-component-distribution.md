# Native component images

MusterOffice owns native compilation and the SDK/worker compatibility check.
A receiving application owns fonts, settings, permissions, jobs, storage, and its
production rollout. A worker remains a child process embedded in that host; OCI
transport does not turn the kernel into a network service.

## Immutable inputs and qualification

The existing `musteroffice.release/1` integration inventory can contain several
native targets and experimental WASM playback. Its development eligibility does
not qualify all of those independent execution surfaces for deployment.

`musteroffice.native-component/1` is a separate native-only distribution contract.
It binds one integration inventory, its exact private Cargo archives and indexes,
a Linux x64 worker, and two independently generated evidence sets:

1. The clean-source Linux builder compiles pinned native dependencies and the Rust
   worker, runs native execution/lifecycle tests, and records ELF identity, dynamic
   dependencies, toolchains, test logs and source revision.
2. `tools/release/check.py` compiles a fresh consumer from the actual registry SDK
   archives, invokes the paired worker, exports PPTX and previews, then reopens and
   inspects the stored result. Its receipt binds the complete integration release.

`tools/release/native-image.py` checks those pins and seals only the native files
and their evidence. Profile `linux-native-presentation/1` records this bounded
qualification. Browser playback, Office/WPS interoperability and the receiving
product's end-to-end acceptance remain explicitly unproven by this profile.
Neither the development release nor a worker build alone grants that qualification.

## Transport and consumption

The emitted Docker context uses a scratch artifact image containing
`/opt/musteroffice/`. It has no daemon or entry point. Publish through an explicitly
authorized registry. Receivers pin the OCI digest and `native-component.json`
digest, verify every inventory entry, and compare `release.json` against their
compiled SDK pin before consuming the component.

A product builder can use the sealed `registry/` as Cargo's standard local-registry
source replacement for its canonical private registry identity. This is an
immutable build dependency fetched from the component image, not a dependency on
a developer checkout or receiving product's already-running registry API. Cargo's
lockfile and archive checksums stay authoritative. The runtime image receives only
the native worker and required evidence, not the SDK source, compiler or credentials.

The host must provide separately pinned fonts/settings, validate its configured
worker digest, and execute its own product acceptance before rolling out. A host
must not infer that the optional browser playback is qualified because this native
image was accepted, or weaken lifecycle, capacity or delivery checks to deploy it.
