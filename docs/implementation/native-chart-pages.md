# Native chart page compilation

The resource page compiler must paint chart graphic frames in the same source
order and transform/opacity scope as shape, table and image content. Chart parts
and embedded workbooks remain unchanged. A chart is never converted to an author
document, synthetic slide objects, or a bitmap.

The page resource stage binds charts through the existing package/chart queries,
resolves declared styles, computes bounded local chart paths and shapes labels
with the caller's explicit font manifest. It retains these local paths with the
real graphic-frame identity and chart-part declaration ordinals. Retained frames
reuse the same paths and recertify placement for every viewport/transform.

Initial page profile: two-dimensional clustered bars (horizontal and vertical),
lines, pie and doughnut charts with cache snapshots, declared solid paints,
linear category/value axes, axis labels, gridlines, data labels and legends.
Circular geometry uses the existing bounded circular chart compiler. Cartesian
layout has explicit limits and a recorded numerical error budget. Unsupported
plots, combinations, effects, date/log scales and unrecognized declarations fail
with the source chart part and ordinal; they are not silently omitted.

Preparation and aggregate budgets precede backend calls. No host font discovery,
external workbook refresh, browser drawing, or network access enters the kernel.
Native export, editor resource pages and WASM playback share this implementation.
Tests must cover source binding, sparse/invalid data, negative/zero values,
unsupported semantics, cancellation, paint order and retained-page equivalence.
Template qualification additionally requires real product import of every page
and visual inspection. Package assembly and product adoption are separate gates.

The initial renderer requires explicit solid colors and label typography;
text fallback remains entirely in the caller's manifest. Bottom legends,
unrotated single-line labels, linear axes at left/bottom and cluster grouping
are supported. Manual layouts, composite labels, line markers, style-dependent
implicit chart colors and unsupported extension content fail admission. This is
not a claim of full Excel chart layout coverage. Default line labels resolve
collisions locally; explicit top labels stay above their data point.

Chart layout is admitted before page geometry/text preflight. Only after the
whole page is admitted may chart shaping call the font component. Chart and
ordinary text share the same resident font session and aggregate work budget.
Retained chart paths have a separate 64 MiB logical storage bound, reported in
preparation metadata, and remain owned by the real source frame. No decoder is
called to turn charts into images. Source caches remain snapshots; embedded
workbooks are preserved without pretending they were recalculated.

Validation on 2026-10-04: 574 source, compile and PPTX regression tests passed;
strict library Clippy and generated TypeScript checks passed. All 28 owned
professional-template pages rendered natively. For their eight chart pages,
the Native and WASM resource-page APIs produced identical complete metadata
and RGBA bytes with the same explicit fonts and settings. This verifies those
inputs, not general Office/WPS interoperability or product activation.
