# Native presentation reliability repair — 2026-09-30

## Ownership and behavior

The kernel owns image layout, decoding, editable document computation and PPTX
semantics. The product owns authorization, storage, task lifecycle, system
application opening and final visual-review workflow. No product image shrinking,
content deletion, format fallback or relaxed pixel budget is introduced.

### Image working set

Compressed file length is not RGBA working-set size. The reproduced page uses
4096×2304, 4096×2732 and 4096×2304 JPEGs: 114.6875 MiB of full-resolution RGBA.
The source files total approximately 1.46 MiB. Keeping all those original pixel
planes for a 1280×720 preview is unnecessary.

Resource preparation resolves all visible uses before decoding. Stretch demand
includes group placement, crop and viewport scale, takes the largest use of each
shared image, and conservatively requests two samples per device pixel. Nearest
sampling and tiled fills retain their exact source grid. JPEG uses a sufficient
native DCT grid before allocating RGBA; PNG retains one bounded decode scratch
plane, filters, then releases it. Original encoded bytes, orientation, density,
source identity and PPTX embedding remain intact. Crop, placement and source-domain
coordinates are rebound to the sampled grid with outward numeric error bounds.
The existing image and page budgets still apply.

Retained playback recompiles brushes against the actual retained sample grid.
Its immutable timing graph determines which owners can rotate or scale; those
owners and descendants of animated groups retain exact source pixels. Deduplicated
resources take the strictest use. Static, translation-only, visibility and opacity
uses retain viewport sampling. A resource plan without a bounded timing graph
also keeps exact pixels. This avoids degrading a later enlarged frame; animated
exact-grid resources still obey the same aggregate budget.

For the reproduced three-image placement the sampled grids are 1024×576,
1024×683 and 1024×576: 7.16796875 MiB, 93.75% less retained pixel data. This is
pixel-buffer accounting, not a claim about total application RSS. Full-resolution
and sampled process timing includes startup and pipe I/O and is not a pure codec
benchmark. The original files are not recompressed or replaced.

### Accessibility

Authored decorative objects now export the DrawingML decorative extension under
nonvisual properties and import title, description and decorative state. The new
retained-fields-v4 binding profile captures these properties. Historical v1–v3
bindings retain their original semantic interpretation and identity. Unknown
extensions remain subject to the original compatibility checks.

### Bounded creation

Compact composition accepts ordered native pictures alongside native shapes and
text. `append` lowers a complete page batch into the ordinary transaction engine:
resource declarations, page order, object IDs, base revision, cancellation and
all-or-nothing validation share the existing document authority. It does not
create another document format or mutable state owner. Strict request parsing
can return bounded structural path/reason feedback while rejecting duplicate keys.

## Verification

- 353 existing portable tests passed across common, image, operations and PPTX.
- Compact append tests cover ordering, resources, duplicate IDs, stale revision,
  cancellation and atomicity. Decorative export/import preserves accessibility.
- Dedicated multi-4K and shared cropped-image tests cover aggregate budget and
  largest-use planning. Exact-grid mode retains its previous budget rejection.
- 16 focused sampling/playback regressions passed, including retained command
  equality, decode-once reuse, crop, group animation and shared-resource bounds.
- 88 actual native/WASM decode pairs, including PNG/JPEG/orientation fixtures and
  the three original 4K assets, have identical metadata and pixel bytes.
- The original failed 11-page revision was replayed with all seven source image
  hashes and all 94 decorative attributes preserved. Export plus independent
  inspection completed in 3.63 seconds with the final pin; the later image-budget-failing revision
  completed in 2.56 seconds. These local command timings exclude model writing,
  image generation, product startup and database work. One measurement per input,
  on Apple M4 Max / 36 GiB / macOS 26.2, with original 22,131,464-byte font bundle
  and concurrent development work; OS caches were not cleared. These are replay
  observations, not statistically established latency targets.
- All 11 exported pages were independently rendered from PPTX in native and WASM;
  inspection reports, text measurements and page RGBA match exactly.
- The final public playback bundle in an isolated Worker passed 22 real retained
  frames against native rendering, incremental sampling equality and six invalid
  delivery input rejections followed by successful reuse.
- Private replay inputs and full logs remain in `.codex-work/ppt-system-repair`.
  They are not release fixtures or uploaded user material.

## Limits

Cross-runtime identity is not PowerPoint/WPS application acceptance. The original
model-authored document itself has a misplaced second image on page 9, oversized
empty regions and inconsistent layout decisions. Kernel export correctly keeps
those authored coordinates; a successful export cannot certify aesthetic quality.
The product must review final page images and repair the native document. No
claim is made that a complete new model-driven task now finishes in the export
benchmark time, or that every image/object profile is supported.
