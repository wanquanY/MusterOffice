# Browser media dependency

Mediabunny **1.61.3**, upstream commit `9d36fc5779b254904bc609e66f2cd80ed8390b4b`, MPL-2.0. This is the same upstream revision inspected in the video design research. The original MusterOffice adapter is Apache-2.0. No upstream source is modified.

`component.json` pins the npm integrity, upstream archive, and every source input. The npm package's `src/` omits the sibling `shared/` sources; it is not the complete corresponding source. The SDK builder therefore compiles the hash-verified **upstream source**, including `shared/`, and includes those readable sources and license in its output. It does not compile an unchecked npm `dist/` tree.

Prepare the reviewed source archive (normal TLS download from `sourceArchive` first):

```sh
python3 tools/video-browser-sdk/prepare.py /absolute/path/to/mediabunny-source.tar.gz
node tools/video-browser-sdk/build.mjs build .codex-work/my-browser-sdk
node tools/video-browser-sdk/build.mjs verify .codex-work/my-browser-sdk MANIFEST_SHA256
```

Build is offline, requires a new destination, checks source hashes before and after compilation, and emits a complete file inventory. The application must pin the manifest externally and validate every file before consuming it. It must also retain the corresponding source and MPL notices in its distribution closure. This development component has `releaseCleared: false`; it is not a public release or an update to v0.1.0.

TypeScript checks application sources strictly. `skipLibCheck` applies only to declarations because this Mediabunny version's legacy `@types/dom-webcodecs` overlaps TypeScript 7's standard DOM declarations. The public component interfaces expose standard platform frames and MusterOffice types, without importing Mediabunny types or requiring it as a runtime peer.

The adapter owns video credits and shared weighted byte admission instead of upstream's fixed prefetch loop. `mediabunny-platform.ts` is an explicitly checked compatibility seam to this pinned source's internal decoder factory; upgrades must rerun real codec checks. Currently the admission model supports progressive AVC with avcC; other video codecs are not silently passed through an unbudgeted fallback. `cloneFrame()` returns a frame **lease**, whose `close()` releases both the frame and its budget. Sequential audio reads reuse a decoder cursor; backward reads still reconstruct from the actual stream start.

Actual behavior, evidence and unresolved media limits: [implementation record](../../docs/implementation/video-browser-media.md).
