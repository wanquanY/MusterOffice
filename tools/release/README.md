# Immutable integration releases

The kernel repository owns SDK packages, native workers and the browser playback
SDK. A host consumes one `musteroffice.release/1` manifest; it does not vendor kernel
source or interpret rendering internals. This assembler has no host repository
imports, database access, publication credentials or network requests.

## Assemble

Produce and verify the portable library SDK with `tools/sdk/build.py`, and the
WASM SDK with `tools/playback-sdk/build.py`. Build/verify each native worker under
its corresponding target separately. Then bind their exact trusted bytes:

```sh
python3 tools/release/build.py \
  --sdk /absolute/verified-sdk --sdk-sha256 SDK_MANIFEST_SHA256 \
  --playback /absolute/verified-playback --playback-sha256 PLAYBACK_MANIFEST_SHA256 \
  --worker darwin-arm64 /absolute/mo-export-worker WORKER_SHA256 \
  --version 0.1.0-dev.EXAMPLE \
  --registry sparse+https://HOST/REGISTRY/index/ \
  --output /absolute/new-release-directory
```

Repeat `--worker TARGET PATH SHA256` for independently verified targets. No target
is inferred or fabricated. Output must be a new directory, outside the source
inputs. Keep it immutable and outside Git. The assembler prints its manifest
SHA-256, version and actual worker targets. Review and pin that digest at the
receiving repository. The manifest does not authenticate itself.

`release.json` binds protocol versions, original SDK inventory digest, each Cargo
package/index record, worker hash and playback files. Current output is explicitly
`channelEligibility: development`; it does not claim product build, performance,
visual quality, external-editor acceptance or clearance to distribute publicly.

## Cargo distribution contract

`registry/NAME-VERSION.crate` is a deterministic standard Cargo archive; a matching
`registry/index/PREFIX/NAME` is suitable for Cargo's standard local-registry source
replacement. Each archive is independent of this checkout. The normalized Cargo
manifest retains only production dependencies, pins owned crates to the same exact
release version, resolves workspace metadata, and includes owned sources plus
required component data/notices. Source-relative resource paths are preserved
under `crates/NAME/src` inside each package. No compiler build scripts or development
test graph is added. Kernel tests remain the responsibility of the kernel repo.

The host's authenticated sparse registry can serve these same index records and
archive bytes after ingestion. It owns credentials and publication policy. The
registry URL is an explicit consumer input, not a source-code dependency on a host.
A version must never be reused for different bytes; adopt a new version even when
only one constituent changes. A host pins the manifest and Cargo.lock together.

Worker and playback binaries stay separate from private Rust package delivery.
The host checks complete release hashes before publication and consumption. A
native worker cannot be upgraded independently of the SDK compiled into its host.

Cargo versions identify distribution artifacts. `Versions.engine` in a delivery
is the kernel's explicit compatibility identity (`MusterOffice/0.1.0`), not a
Cargo package version. Both the producer and inspector require that exact identity;
renderer identity, worker SHA-256 and all delivery content pins remain mandatory.
A registry repackaging must not make its SDK reject its paired worker's delivery.

## Validation without building the host

```sh
python3 -m unittest discover -s tools/release -p 'test_*.py' -v
```

This covers deterministic Cargo archives and a tiny owned consumer after deleting
its source SDK fixture, including a source-relative embedded resource. It does not
build Musterwork or execute PPT acceptance. A real consuming repository can also
run `cargo metadata --offline --locked` using the explicit local registry to check
its dependency graph without compilation.

Before adoption, execute a real export with the **registry archives** and paired
worker. A successful source-workspace example or `cargo check` alone cannot detect
runtime differences caused by distribution versions:

```sh
python3 tools/release/check.py \
  --release /absolute/release --sha256 RELEASE_MANIFEST_SHA256 \
  --target darwin-arm64 --input /absolute/owned-export-fixture \
  --output /absolute/new-check-directory
```

Input uses the SDK example's `request.json`, `snapshot.json` and `assets.json`;
the export request must pin this worker and its explicit resources/fonts. The
check uses the unchanged public SDK example in a fresh consumer, Cargo's scoped
local registry and offline locked build. It exports and independently verifies
the actual stored bytes. It does not require or inspect a receiving product.

Musterwork's adapter uses its existing Admin component authority for storage,
signing, publication and rollback; see that repository's
`components/musteroffice/README.md` for its deployment and local-link commands.
