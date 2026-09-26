# MusterOffice Rust embedding SDK — development inputs

The `mo-embedded-sdk` facade exposes the same document, operation, delivery and
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

Build consumers outside this immutable directory. Set their target directory
outside it too. Cargo may need its already-locked registry packages; preparation
uses offline resolution and never silently fetches or selects new versions.
The example under `examples/native-export` is a consumer template: copy it to a
separate workspace and adjust only its SDK path dependency. It exercises actual
export and final retained-file inspection, without publishing a product result.
